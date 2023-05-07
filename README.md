# WIP: tzf's Rust port. [![Rust](https://github.com/ringsaturn/tzf-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/ringsaturn/tzf-rs/actions/workflows/rust.yml) [![Documentation](https://docs.rs/tzf-rs/badge.svg)](https://docs.rs/tzf-rs) [![codecov](https://codecov.io/gh/ringsaturn/tzf-rs/branch/main/graph/badge.svg?token=NQFIP9DD86)](https://codecov.io/gh/ringsaturn/tzf-rs)

- Documents: <https://docs.rs/tzf-rs>
- Original Go repo: <https://github.com/ringsaturn/tzf>
- Binary timezone data: <https://github.com/ringsaturn/tzf-rel>
- Geometry: use <https://github.com/ringsaturn/geometry-rs> which is
  <https://github.com/tidwall/geometry>'s Rust port.

## Build options

By default, the binary is built as well. If you don't want/need it, then build like this:

```bash
> cargo build --no-default-features
```

Or put in your `Cargo.toml` file:

```toml
tzf-rs = { version = "0.4.1", default-features = false }
```

## Bindings

- Ruby, see [tzf-rb](https://github.com/HarlemSquirrel/tzf-rb)
- Python, see [tzfpy](https://github.com/ringsaturn/tzfpy)
