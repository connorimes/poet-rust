# POET Rust Wrappers

The `poet-rust` crate provides some abstractions over the `poet-sys` crate,
available at
[https://github.com/libpoet/poet-sys](https://github.com/libpoet/poet-sys).

## Dependencies

The `poet-rust` crate depends on the `poet-sys` crate.

Additionally, you must have the `poet` library installed to the system.

The latest `poet` C library can be found at
[https://github.com/libpoet/poet](https://github.com/libpoet/poet).

## Usage
Add `poet-rust` as a dependency in `Cargo.toml`:

```toml
[dependencies.poet-rust]
git = "https://github.com/libpoet/poet-rust.git"
```
