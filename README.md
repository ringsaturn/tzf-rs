# WIP: tzf's Rust port. [![Rust](https://github.com/ringsaturn/tzf-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/ringsaturn/tzf-rs/actions/workflows/rust.yml)

## Usage

Add to `Cargo.toml`

```toml
tzf-rs = "0.1.1"
```

```rust
use tzf_rs::DefaultFinder;

fn main() {
    let finder = DefaultFinder::new();

    print!("{:?}\n", DefaultFinder.get_tz_name(116.3883, 39.9289));
}
```

## References:

- Original Go repo: <https://github.com/ringsaturn/tzf>
- Binary timezone data: <https://github.com/ringsaturn/tzf-rel>
- Geometry: <https://github.com/ringsaturn/geometry-rs>
  which is <https://github.com/tidwall/geometry>'s Rust port.
