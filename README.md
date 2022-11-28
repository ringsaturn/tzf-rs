# WIP: tzf's Rust port. [![Rust](https://github.com/ringsaturn/tzf-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/ringsaturn/tzf-rs/actions/workflows/rust.yml) [![Documentation](https://docs.rs/tzf-rs/badge.svg)](https://docs.rs/tzf-rs)

## Usage

Add to `Cargo.toml`

```toml
tzf-rs = "0.1.3"
```

```rust
use tzf_rs::DefaultFinder;

fn main() {
    let finder = DefaultFinder::new();

    print!("{:?}\n", finder.get_tz_name(116.3883, 39.9289));
}
```

## References:

- Documents: <https://docs.rs/tzf-rs>
- Original Go repo: <https://github.com/ringsaturn/tzf>
- Binary timezone data: <https://github.com/ringsaturn/tzf-rel>
- Geometry: use <https://github.com/ringsaturn/geometry-rs>
  which is <https://github.com/tidwall/geometry>'s Rust port.
