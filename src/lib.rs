extern crate libc;
extern crate poet_sys;

use libc::{c_void, c_int, c_uint};
use std::ffi::CString;
use std::ptr;
use poet_sys::*;

extern fn apply_cpu_config_wrapper(states: *mut c_void,
                                   num_states: c_uint,
                                   id: c_uint,
                                   last_id: c_uint) {
    unsafe {
        apply_cpu_config(states, num_states, id, last_id)
    }
}

extern fn get_current_cpu_state_wrapper(states: *const c_void,
                                        num_states: c_uint,
                                        curr_state_id: *mut c_uint) -> c_int {
    unsafe {
        get_current_cpu_state(states, num_states, curr_state_id)
    }
}

pub fn default_poet_control_state_t() -> poet_control_state_t {
	poet_control_state_t {
        id: 0,
        speedup: 1.0,
        cost: 1.0,
    }
}

pub fn default_poet_cpu_state_t() -> poet_cpu_state_t {
	poet_cpu_state_t {
        id: 0,
        freq: 0,
        cores: 0,
    }
}

/// Attempt to load control states from a file.
pub fn poet_get_control_states(filename: Option<&CString>) -> Result<Vec<poet_control_state_t>, &'static str> {
    let name_ptr = match filename {
        Some(f) => f.as_ptr(),
        None => ptr::null(),
    };
    let mut states: *mut poet_control_state_t = ptr::null_mut::<poet_control_state_t>();
    unsafe {
        let mut nstates: u32 = 0;
        let res = get_control_states(name_ptr,
                                     &mut states,
                                     &mut nstates);
        if res != 0 {
            return Err("Failed to load control states");
        }
        // clone so we can free C-allocated memory (so user doesn't have to)
        let mut ret = Vec::with_capacity(nstates as usize);
        ret.set_len(nstates as usize);
        ptr::copy_nonoverlapping(states, ret.as_mut_ptr(), nstates as usize);
        libc::free(states as *mut c_void);
        Ok(ret)
    }
}

/// Attempt to load cpu states from a file.
pub fn poet_get_cpu_states(filename: Option<&CString>) -> Result<Vec<poet_cpu_state_t>, &'static str> {
    let name_ptr = match filename {
        Some(f) => f.as_ptr(),
        None => ptr::null(),
    };
    let mut states: *mut poet_cpu_state_t = ptr::null_mut::<poet_cpu_state_t>();
    unsafe {
        let mut nstates: u32 = 0;
        let res = get_cpu_states(name_ptr,
                                 &mut states,
                                 &mut nstates);
        if res != 0 {
            return Err("Failed to load cpu states");
        }
        // clone so we can free C-allocated memory (so user doesn't have to)
        let mut ret = Vec::with_capacity(nstates as usize);
        ret.set_len(nstates as usize);
        ptr::copy_nonoverlapping(states, ret.as_mut_ptr(), nstates as usize);
        libc::free(states as *mut c_void);
        Ok(ret)
    }
}

/// The `POET` struct wraps an underyling C struct.
pub struct POET {
    /// The underlying C struct `poet_state`.
    pub poet: *mut poet_state,
    pub control_states: Vec<poet_control_state_t>,
    pub cpu_states: Vec<poet_cpu_state_t>
}

impl POET {
    /// Attempt to initialize POET and allocate resources in the underlying C struct.
    pub fn new(perf_goal: f64,
               mut control_states: Vec<poet_control_state_t>,
               mut cpu_states: Vec<poet_cpu_state_t>,
               apply_func: Option<poet_apply_func>,
               curr_state_func: Option<poet_curr_state_func>,
               period: u32,
               buffer_depth: u32,
               log_filename: Option<&CString>) -> Result<POET, &'static str> {
        if control_states.len() != cpu_states.len() {
            return Err("Number of control and cpu states don't match");
        }
        // the following necessary cast for None seem to be a bug in Rust coercion
        let apply_func: poet_apply_func = match apply_func {
            Some(p) => p,
            None => apply_cpu_config_wrapper,
        };
        let curr_state_func: poet_curr_state_func = match curr_state_func {
            Some(p) => p,
            None => get_current_cpu_state_wrapper,
        };
        let log_ptr = match log_filename {
            Some(l) => l.as_ptr(),
            None => ptr::null(),
        };
        let poet = unsafe {
            let num_states = control_states.len() as u32;
            poet_init(perf_goal,
                      num_states, control_states.as_mut_ptr(), cpu_states.as_mut_ptr(),
                      apply_func, curr_state_func,
                      period, buffer_depth, log_ptr)
        };
        if poet.is_null() {
            return Err("Failed to instantiate POET object");
        }
        Ok(POET {
        	poet: poet,
        	control_states: control_states,
        	cpu_states: cpu_states,
        })
    }

    /// Call at every iteration - at specified periods this function will (potentially) order
    /// changes to system or application state to try and meet timing constraints.
    pub fn apply_control(&mut self, tag: u64, window_rate: f64, window_power: f64) {
        unsafe {
            poet_apply_control(self.poet, tag, window_rate, window_power);
        }
    }
}

impl Drop for POET {
    /// Cleanup POET and deallocate resources in the underlying C struct.
    fn drop(&mut self) {
        unsafe {
            poet_destroy(self.poet);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use libc::{c_void, c_uint};
    use std::ffi::CString;

    #[test]
    fn test_basic() {
        let control_states = vec![default_poet_control_state_t()];
        let cpu_states = vec![default_poet_cpu_state_t()];
        let mut poet = POET::new(100.0,
                                 control_states, cpu_states,
                                 None, None,
                                 20u32, 1u32, None).unwrap();
        poet.apply_control(0, 1.0, 1.0);
    }

    #[test]
    fn test_control_cpu_files_with_log() {
        let control_states = poet_get_control_states(Some(&CString::new("test/control_config").unwrap())).unwrap();
        let cpu_states = poet_get_cpu_states(Some(&CString::new("test/cpu_config").unwrap())).unwrap();
        let mut poet = POET::new(100.0,
                                 control_states, cpu_states,
                                 None, None,
                                 20u32, 1u32, Some(&CString::new("poet.log").unwrap())).unwrap();
        poet.apply_control(0, 1.0, 1.0);
    }

    #[test]
    fn test_rust_callbacks() {
        let control_states = vec![default_poet_control_state_t()];
        let cpu_states = vec![default_poet_cpu_state_t()];
        let mut poet = POET::new(100.0,
                                 control_states, cpu_states,
                                 Some(dummy_apply), Some(dummy_curr_state),
                                 20u32, 1u32, None).unwrap();
        for i in 0..50 {
            poet.apply_control(i, 1.0, 1.0);
        }
    }

    extern fn dummy_apply(_states: *mut c_void,
                          _num_states: c_uint,
                          _id: c_uint,
                          _last_id: c_uint) {
        // do nothing
        println!("Received apply call");
    }

    extern fn dummy_curr_state(_states: *const c_void,
                               _num_states: c_uint,
                               _curr_state_id: *mut c_uint) -> i32 {
        println!("Received curr state call");
        unsafe {
            // this is actually an invalid value, but forces the apply function to be called
            *_curr_state_id = _num_states;
        }
        0
    }

}
