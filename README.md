# tzf-rs: a fast timezone finder for Rust. [![Rust](https://github.com/ringsaturn/tzf-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/ringsaturn/tzf-rs/actions/workflows/rust.yml) [![Documentation](https://docs.rs/tzf-rs/badge.svg)](https://docs.rs/tzf-rs)

![Time zone map of the world](https://github.com/ringsaturn/tzf/blob/gh-pages/docs/tzf-social-media.png?raw=true)

> [!NOTE]
>
> This package uses simplified shape data so it is not entirely accurate around
> the border.

## Build options

By default, the binary is built as well. If you don't want/need it, you can omit
the default features and build like this:

```bash
cargo build --no-default-features
```

Or add in the below way:

```bash
cargo add tzf-rs --no-default-features
```

## Best Practices

It's expensive to init tzf-rs's `Finder`/`FuzzyFinder`/`DefaultFinder`, so
please consider reusing instances or creating one as a global variable. Below is
a global variable example:

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

For reuse,
[`racemap/rust-tz-service`](https://github.com/racemap/rust-tz-service) provides
a good example.

A Redis protocol demo could be used here:
[`ringsaturn/redizone`](https://github.com/ringsaturn/redizone).

## Performance

The tzf-rs package is intended for high-performance geospatial query services,
such as weather forecasting APIs. Most queries can be returned within a very
short time, averaging around 3,000 nanoseconds (about 1,000ns slower than with
Go repo `tzf`. I will continue improving this - you can track progress
[here](https://github.com/ringsaturn/geometry-rs/issues/3)).

Here is what has been done to improve performance:

1. Using pre-indexing to handle most queries takes approximately 1000
   nanoseconds.
2. Using a finely-tuned Ray Casting algorithm package
   [`ringsaturn/geometry-rs`](https://github.com/ringsaturn/geometry-rs) to
   verify whether a polygon contains a point.

That's all. There are no black magic tricks inside the tzf-rs.

Below is a benchmark run on global cities(about 14K), and avg time is about
3,000 ns per query:

```txt
test benches_default::bench_default_finder_random_city ... bench:       2,870 ns/iter (+/- 182)
```

| Criterion result | Pic                                                                                       |
| ---------------- | ----------------------------------------------------------------------------------------- |
| PDF              | ![](https://raw.githubusercontent.com/ringsaturn/tzf-rs/main/assets/pdf_small.svg)        |
| Regression       | ![](https://raw.githubusercontent.com/ringsaturn/tzf-rs/main/assets/regression_small.svg) |

You can view more details from latest benchmark from
[GitHub Actions logs](https://github.com/ringsaturn/tzf-rs/actions/workflows/rust.yml).

## References

I have written an article about the history of `tzf`, its Rust port, and its
Rust port's Python binding; you can view it
[here](https://blog.ringsaturn.me/en/posts/2023-01-31-history-of-tzf/).

- Original Go repo: [`ringsaturn/tzf`](https://github.com/ringsaturn/tzf)
- Binary timezone data:
  [`ringsaturn/tzf-rel`](https://github.com/ringsaturn/tzf-rel)
- Geometry: use
  [`ringsaturn/geometry-rs`](https://github.com/ringsaturn/geometry-rs) which is
  [`tidwall/geometry`](https://github.com/tidwall/geometry)'s Rust port.
- Continuous Benchmark compared with other packages:
  [`ringsaturn/tz-benchmark`](https://github.com/ringsaturn/tz-benchmark)

### Bindings

- Ruby, see [`HarlemSquirrel/tzf-rb`](https://github.com/HarlemSquirrel/tzf-rb)
- Python, see [`ringsaturn/tzfpy`](https://github.com/ringsaturn/tzfpy)

## LICENSE

This project is licensed under the [MIT license](./LICENSE). The data is
licensed under the
[ODbL license](https://github.com/ringsaturn/tzf-rel/blob/main/LICENSE), same as
[`evansiroky/timezone-boundary-builder`](https://github.com/evansiroky/timezone-boundary-builder)
