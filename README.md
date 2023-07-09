# WIP: tzf's Rust port. [![Rust](https://github.com/ringsaturn/tzf-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/ringsaturn/tzf-rs/actions/workflows/rust.yml) [![Documentation](https://docs.rs/tzf-rs/badge.svg)](https://docs.rs/tzf-rs) [![codecov](https://codecov.io/gh/ringsaturn/tzf-rs/branch/main/graph/badge.svg?token=NQFIP9DD86)](https://codecov.io/gh/ringsaturn/tzf-rs)

- Documents: <https://docs.rs/tzf-rs>
- Original Go repo: <https://github.com/ringsaturn/tzf>
- Binary timezone data: <https://github.com/ringsaturn/tzf-rel>
- Geometry: use <https://github.com/ringsaturn/geometry-rs> which is
  <https://github.com/tidwall/geometry>'s Rust port.

## Build options

By default, the binary is built as well. If you don't want/need it, then build
like this:

```bash
cargo build --no-default-features
```

Or add in the below way:

```bash
cargo add tzf-rs --no-default-features
```

## Best practice

It's expensive to init tzf-rs's Finder/FuzzyFinder/DefaultFinder, please
consider reuse it or as a global var. Below is a global var example:

```rust
use lazy_static::lazy_static;
use tzf_rs::DefaultFinder;

lazy_static! {
    static ref FINDER: DefaultFinder = DefaultFinder::new();
}

fn main() {
    print!("{:?}\n", FINDER.get_tz_name(116.3883, 39.9289));
    print!("{:?}\n", FINDER.get_tz_names(116.3883, 39.9289));
}
```

For reuse it,
[`racemap/rust-tz-service`](https://github.com/racemap/rust-tz-service) is a
good example.

A Redis protocol demo could be used here:
[redizone](https://github.com/ringsaturn/redizone).

## Bindings

- Ruby, see [tzf-rb](https://github.com/HarlemSquirrel/tzf-rb)
- Python, see [tzfpy](https://github.com/ringsaturn/tzfpy)
