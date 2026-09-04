# tzf-rs: a fast timezone finder for Rust. [![Rust](https://github.com/ringsaturn/tzf-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/ringsaturn/tzf-rs/actions/workflows/rust.yml) [![Documentation](https://docs.rs/tzf-rs/badge.svg)](https://docs.rs/tzf-rs) [![Crates.io Version](https://img.shields.io/crates/v/tzf-rs)](https://crates.io/crates/tzf-rs) [![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2Fringsaturn%2Ftzf-rs.svg?type=shield)](https://app.fossa.com/projects/git%2Bgithub.com%2Fringsaturn%2Ftzf-rs?ref=badge_shield)

![Time zone map of the world](https://github.com/ringsaturn/tzf/blob/gh-pages/docs/tzf-social-media.png?raw=true)

- Released documentation: [docs.rs/tzf-rs](https://docs.rs/tzf-rs)
- Latest documentation(not released yet):
  [ringsaturn.github.io/tzf-rs](https://ringsaturn.github.io/tzf-rs/tzf_rs/)
- Try it online: [tzf-web](https://ringsaturn.github.io/tzf-web/)

> [!NOTE]
>
> **Version 2 is protobuf-free.** The data source is the TZF embedded binary
> format (`.tzb`) shipped by
> [tzf-dist](https://github.com/ringsaturn/tzf-dist), and the public surface
> is two finder types: `DefaultFinder` and `EmbeddedFinder`. See
> [Migrating from v1](#migrating-from-v1).

## Quick start

```bash
cargo add tzf-rs
```

```rust
use tzf_rs::DefaultFinder;

fn main() {
    let finder = DefaultFinder::new();
    // Please note coords are lng-lat.
    println!("{:?}", finder.get_tz_name(116.3883, 39.9289));
    println!("{:?}", finder.get_tz_names(116.3883, 39.9289));
}
```

By default the `tzf` CLI binary is built as well. If you don't want/need it,
you can omit the default features and build like this:

```bash
cargo build --no-default-features --features bundled
```

## Finders

- **`DefaultFinder`** — the recommended general-purpose finder. Expands the
  file's geometry into materialized polygons at load time. `get_tz_name`
  answers most queries from the preindex tiles in ~100 ns and falls back to
  exact point-in-polygon (YStripes-accelerated ray casting) for boundary
  cases; `get_tz_names` is always polygon-exact.
- **`EmbeddedFinder`** — the low-memory finder. Queries the `.tzb` bytes in
  place, without expanding geometry: total footprint is roughly the ~4 MB
  file plus ~1 KB of state. Queries are microseconds instead of hundreds of
  nanoseconds on boundary cases; results are identical to `DefaultFinder`.

Both also load caller-supplied bytes: `DefaultFinder::from_tzb` /
`EmbeddedFinder::from_tzb`.

## Best Practices

It's expensive to init tzf-rs's `DefaultFinder`/`EmbeddedFinder`, so please
consider reusing instances or creating one as a global variable:

```rust
use std::sync::LazyLock;
use tzf_rs::DefaultFinder;

static FINDER: LazyLock<DefaultFinder> = LazyLock::new(DefaultFinder::new);

fn main() {
    // Please note coords are lng-lat.
    println!("{:?}", FINDER.get_tz_name(116.3883, 39.9289));
    println!("{:?}", FINDER.get_tz_names(116.3883, 39.9289));
}
```

For reuse,
[`racemap/rust-tz-service`](https://github.com/racemap/rust-tz-service) provides
a good example.

A Redis protocol demo could be used here:
[`ringsaturn/redizone`](https://github.com/ringsaturn/redizone).

### Setup 100% Accurate Lookup

By default, tzf-rs uses simplified shape data. The error around borders is
small and bounded: every simplified boundary stays within ~111 m of the
full-precision border. See [Accuracy](#accuracy) for measured numbers. If you
need 100% accurate lookup, use the full-precision dataset (git-only, ~14 MB):

```toml
tzf-rs = { git =  "https://github.com/ringsaturn/tzf-rs", rev = "v{X}.{Y}.{Z}", features = ["full"], default-features = false }
```

```rust,ignore
use tzf_rs::DefaultFinder;

fn main() {
    let finder = DefaultFinder::new_full();
    println!("{}", finder.timezonenames().len());
    let tz_name = finder.get_tz_name(139.767125, 35.681236);
    println!("tz_name: {}", tz_name);
}
```

**This setup requires more time and memory to build the `DefaultFinder`.**

## Advanced Usage - Export GeoJSON

> [!NOTE]
>
> This feature is designed for **data visualization purposes** and I can't
> guarantee the performance when using it in high-performance scenarios. Please
> do proper performance tests and necessary optimizations before using it in
> high performace production, for example caching the exported GeoJSON data or
> push to CDN.

It's a common use case make some visualization of timezone boundaries. For this
purpose, tzf-rs provides methods to export specific timezone polygons as
GeoJSON format.

To enable this feature, you need to build tzf-rs with `export-geojson` feature:

```toml
tzf-rs = { version = "{version}", features = ["export-geojson"]}
```

Then you can use the following methods:

```rust
// examples/query_tokyo.rs
use tzf_rs::DefaultFinder;

fn main() {
    let default_finder = DefaultFinder::new();
    let lng = 139.6917;
    let lat = 35.6895;

    let tz_name = default_finder.get_tz_name(lng, lat).to_owned();
    println!(
        "The timezone at longitude {}, latitude {} is: {}",
        lng, lat, tz_name
    );

    // Get the polygon boundary for the timezone.
    if let Some(boundary_file) = default_finder.get_tz_geojson(&tz_name) {
        // It's a GeoJSON FeatureCollection whose features contain
        // "MultiPolygon" geometry for the timezone.
        println!("Found GeoJSON feature for timezone: {}", tz_name);
        let mut polygons: usize = 0;
        for feature in boundary_file.features {
            polygons += feature.geometry.coordinates.len();
        }
        println!(
            "Total number of polygons in feature collection: {}",
            polygons
        );
    }
}
```

```bash
cargo run --example query_tokyo --features export-geojson
```

`EmbeddedFinder` exports the same GeoJSON, decoding only the requested
timezone's rings from the file on demand.

For now, tzf-rs' binding in Wasm, named
[tzf-wasm](https://github.com/ringsaturn/tzf-wasm), has exported this feature
and it has been deployed to the [tzf-web](https://ringsaturn.github.io/tzf-web/)
for online usage.

## Migrating from v1

v1 loaded protobuf artifacts (`CompressedTopoTimezones`, `PreindexTimezones`);
those artifacts are no longer published, and v2 removes every protobuf-typed
API. Mappings:

| v1                                       | v2                                                        |
| ---------------------------------------- | --------------------------------------------------------- |
| `DefaultFinder::new()`                   | `DefaultFinder::new()` (unchanged call sites)             |
| `DefaultFinder::new_full()`              | `DefaultFinder::new_full()` (unchanged call sites)        |
| `Finder` (polygon-only)                  | `DefaultFinder` (`get_tz_names` stays polygon-exact)      |
| `FuzzyFinder` (tile-only)                | removed — the preindex is the fast path inside every finder |
| `Finder::from_compressed_topo(pb)`       | `DefaultFinder::from_tzb(bytes)`                          |
| `FuzzyFinder::from_pb(pb)`               | removed, no replacement                                   |
| `FinderOptions` / `new_with_options`     | removed — YStripes is always on                           |
| `finder.finder.get_tz_geojson(...)`      | `finder.get_tz_geojson(...)`                              |
| `FuzzyFinder` tile-bbox GeoJSON          | removed, no replacement                                   |

Behavior changes:

- `get_tz_names` results are now sorted lexicographically.
- `get_tz_name` on `DefaultFinder` answers from the preindex tile when one
  covers the point (v1 `DefaultFinder` semantics; v1 `Finder` users who need
  polygon-exact multi-results use `get_tz_names`).
- New: `EmbeddedFinder`, an in-place low-memory mechanism (~4 MB total).

## Accuracy

The Douglas-Peucker simplification uses an epsilon of 0.001 degrees, which
caps boundary displacement at roughly 111 m by construction. Measured against
the full-precision 2026c dataset with `tzf`'s `internal/cmd/borderchange`
(spherical model, certified via Lipschitz interval subdivision):

| Metric                                            |                        Result |
| ------------------------------------------------- | ----------------------------: |
| Certified maximum boundary displacement           | 111.2 m (+1.0 m tolerance)    |
| Boundary length displaced more than 100 m         | 0.41%                         |
| Boundary length displaced more than 500 m         | 0%                            |
| Total mis-assigned area                           | 16,828 km² (~0.003% of Earth) |
| Mis-assigned area within 100 m of the true border | 92.8%                         |

See [`BORDER_CHANGE.md`](https://github.com/ringsaturn/tzf/blob/main/BORDER_CHANGE.md)
in the `tzf` repository for the complete evaluation results.

Only queries that land within ~111 m of a timezone border can differ from the
full-precision result, and most of that band is far narrower. If your use case
is sensitive inside that band, enable the `full` feature and use
`DefaultFinder::new_full()`.

## Performance

The tzf-rs package is intended for high-performance geospatial query services,
such as weather forecasting APIs. Most queries can be returned within a very
short time, averaging around 100-300 nanoseconds with `DefaultFinder`.

Here is what has been done to improve performance:

1. Using the simplified dataset by default.
2. Using pre-indexing (the `.tzb` FUZZY section) to handle most queries in
   about 100 nanoseconds.
3. Using a finely-tuned Ray Casting algorithm package
   [`ringsaturn/geometry-rs`](https://github.com/ringsaturn/geometry-rs) to
   verify whether a polygon contains a point.
   - Using YStripes(inspired by Josh Baker's
     [`tg`](https://github.com/tidwall/tg)'s ) to accerate polygon queries. This
     polygon index works when the pre-indexing missing, especially for queries
     around the border.
   - Also the dense 1°×1° grid index carried by the `.tzb` file to quickly
     find candidate polygons, inspired by Aaron Roney's
     [rtz](https://github.com/twitchax/rtz).

That's all. There are no black magic tricks inside the tzf-rs.

Benchmark numbers (Apple M3 Max, bundled lite dataset, `cargo bench`):

| Target         | Scenario                  | Median estimate |
| -------------- | ------------------------- | --------------: |
| DefaultFinder  | random city               |         ~260 ns |
| DefaultFinder  | edge city (preindex miss) |         ~400 ns |
| EmbeddedFinder | random city               |         ~1.6 µs |
| EmbeddedFinder | edge city (preindex miss) |         ~3.9 µs |
| DefaultFinder  | open (`new()`)            |          ~13 ms |
| EmbeddedFinder | open (`new()`)            |           ~2 ms |

tzf-rs consumes the `.tzb` profile only. The `.tzm` memory image the Go
runtime uses exists for zero-copy ring aliasing, which geometry-rs's owned
polygon storage cannot exploit — measured here it saved ~3 ms of open time
while costing more memory, so it is not supported.

Peak RSS (macOS, `make memory`): ~44 MiB for `DefaultFinder` (v1 needed
~82 MiB), ~6 MiB for `EmbeddedFinder`.

You can view more details from latest benchmark from
[GitHub Actions logs](https://github.com/ringsaturn/tzf-rs/actions/workflows/rust.yml).

## References

I have written an article about the history of `tzf`, its Rust port, and its
Rust port's Python binding; you can view it
[here](https://blog.ringsaturn.me/en/posts/2023-01-31-history-of-tzf/).

- Original Go repo: [`ringsaturn/tzf`](https://github.com/ringsaturn/tzf)
- Binary timezone data (`.tzb` / `.tzm`):
  [`ringsaturn/tzf-dist`](https://github.com/ringsaturn/tzf-dist)
- Geometry: use
  [`ringsaturn/geometry-rs`](https://github.com/ringsaturn/geometry-rs) which is
  [`tidwall/geometry`](https://github.com/tidwall/geometry)'s Rust port.
- Continuous Benchmark compared with other packages:
  [`ringsaturn/tz-benchmark`](https://github.com/ringsaturn/tz-benchmark)

See [Project tzf](https://project-tzf.ringsaturn.me/docs/getting-started/) for
more information.

### Bindings

- Ruby, see [`HarlemSquirrel/tzf-rb`](https://github.com/HarlemSquirrel/tzf-rb)
- Python, see [`ringsaturn/tzfpy`](https://github.com/ringsaturn/tzfpy)
- Wasm, see [`ringsaturn/tzf-wasm`](https://github.com/ringsaturn/tzf-wasm)
- PostgreSQL extension, see
  [`ringsaturn/pg-tzf`](https://github.com/ringsaturn/pg-tzf)

## Command line

The binary helps in debugging tzf-rs and using it in (scripting) languages
without bindings. Either specify the coordinates as parameters to get a single
time zone, or to look up multiple coordinates efficiently specify the ordering
and pipe them to the binary one pair of coordinates per line.

```shell
tzf --lng 116.3883 --lat 39.9289
echo -e "116.3883 39.9289\n116.3883, 39.9289" | tzf --stdin-order lng-lat
```

If you are using Nixpkgs, you can install the `tzf` command line tool, please
see more in
[Nixpkgs](https://search.nixos.org/packages?channel=unstable&type=packages&query=tzf-rs).

## LICENSE

This project is licensed under the [MIT license](./LICENSE) and
[Anti CSDN License](./LICENSE_ANTI_CSDN.md)[^anti_csdn]. The data is licensed
under the
[ODbL license](https://github.com/ringsaturn/tzf-dist/blob/main/LICENSE_DATA),
same as
[`evansiroky/timezone-boundary-builder`](https://github.com/evansiroky/timezone-boundary-builder)

[^anti_csdn]:
    This license is to prevent the use of this project by CSDN, has no
    effect on other use cases.

[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2Fringsaturn%2Ftzf-rs.svg?type=large)](https://app.fossa.com/projects/git%2Bgithub.com%2Fringsaturn%2Ftzf-rs?ref=badge_large)
