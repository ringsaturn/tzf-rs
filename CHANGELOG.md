# Changelog

## v2.0.0 (2026-09-11)

tzf-rs v2 is protobuf-free. The data source is the TZF embedded binary format
(`.tzb`) shipped by
[tzf-dist](https://github.com/ringsaturn/tzf-dist); the protobuf artifacts of
the v1 line are no longer published, so staying on v1 means staying on its
last data release.

MSRV: Rust 1.88 (edition 2024 plus let-chains), declared as `rust-version` in
`Cargo.toml`.

### Public surface

Two finder types, mirroring the Go `tzf/v2` design:

- `DefaultFinder` — the recommended finder. Loads a `.tzb` by expanding its
  geometry into geometry-rs polygons (`new()`, `new_full()`, `from_tzb`).
  `get_tz_name` answers from the FUZZY preindex tiles first and falls back
  to exact point-in-polygon; `get_tz_names` stays polygon-exact.
- `EmbeddedFinder` — the low-memory finder: queries `.tzb` bytes in place
  (~4 MB total for the bundled lite data), FUZZY-first with the
  compressed-geometry scan as fallback. `from_tzb` accepts `&'static [u8]`
  or `Vec<u8>`.

Both expose `get_tz_name`, `get_tz_names`, `timezonenames` and
`data_version`, and — under `export-geojson` — `to_geojson`,
`get_tz_geojson`, `to_preindex_geojson` and `get_tz_preindex_geojson`.
Alongside them the crate exports `Error` (new in v2), `deg2num`, and the
GeoJSON types (`BoundaryFile`, `FeatureItem`, `GeometryDefine`,
`PropertiesDefine`, `PolygonCoordinates`, `MultiPolygonCoordinates`) under
`export-geojson`. That is the whole public API.

### Removed

- Every protobuf-typed API: `Finder::from_pb`, `Finder::from_compressed_topo`
  (and their `*_with_options` variants), `FuzzyFinder::from_pb`, the `pbgen`
  module, the `prost` / `prost-build` dependencies, `anyhow`, `bytes`, and the
  protobuf build machinery (`build.rs`, `buf.yaml`, the `pb/` directory).
- The `Finder` and `FuzzyFinder` types. `DefaultFinder` covers `Finder`
  (`get_tz_names` is polygon-exact); tile-only lookup is no longer a public
  mechanism in any form — the preindex is the internal fast path inside every
  finder.
- `FinderOptions` and the `*_with_options` constructors — the YStripes index
  is always enabled.
- `revert_timezones` (it took a protobuf type).

### Migration

| v1                                        | v2                                                          |
| ----------------------------------------- | ----------------------------------------------------------- |
| `DefaultFinder::new()`                    | unchanged                                                    |
| `DefaultFinder::new_full()`               | unchanged (`full` feature, git-only data)                    |
| `DefaultFinder::{get_tz_name,get_tz_names,timezonenames,data_version}` | unchanged                        |
| `Finder` (polygon-only)                   | `DefaultFinder` (`get_tz_names` stays polygon-exact)         |
| `FuzzyFinder` (tile-only)                 | removed — the preindex is the fast path inside every finder  |
| `Finder::from_compressed_topo(pb)` / `from_pb(pb)` | `DefaultFinder::from_tzb(&[u8])`                    |
| `FuzzyFinder::from_pb(pb)`                | removed — no separate tile-only finder                       |
| `FinderOptions` / `*_with_options`        | removed — YStripes is always on                              |
| `tzf_rs::pbgen`                           | removed — no protobuf types in the public API                |
| `tzf_rs::revert_timezones(&pb)`           | removed — took a protobuf type                               |
| `finder.finder.get_tz_geojson(...)`       | `finder.get_tz_geojson(...)`                                 |
| `FuzzyFinder::to_geojson()`               | `DefaultFinder::to_preindex_geojson() -> Option<BoundaryFile>` |
| `FuzzyFinder::get_tz_geojson(name) -> Option<FeatureItem>` | `DefaultFinder::get_tz_preindex_geojson(name) -> Option<BoundaryFile>` |
| feature `bundled` (pb lite data)          | feature `bundled` (lite `.tzb`)                              |
| feature `full` (pb full data, git-only)   | feature `full` (full `.tzb`, still git-only)                 |
| features `clap`, `export-geojson`         | unchanged                                                    |
| —                                         | new: `EmbeddedFinder`, `tzf_rs::Error`                       |

### Behavior changes

- `get_tz_names` results are sorted lexicographically (matching the Go v2
  finder).
- GeoJSON exports omit the duplicated junction vertices the protobuf
  expansion retained (zero-length segments; query results are unaffected).
- Malformed data now surfaces as `Err(tzf_rs::Error)` from the byte
  constructors instead of an empty default finder: files are CRC-checked and
  structurally validated at open.
- The preindex GeoJSON exports return a `BoundaryFile` (FeatureCollection);
  v1's `FuzzyFinder::get_tz_geojson` returned a bare `FeatureItem`.

### Data features

- `bundled` (default): lite `.tzb` from tzf-dist on crates.io (~4 MB).
- `full`: full-precision `.tzb` (~14 MB), enables `new_full()`. The full
  dataset is git-only: it exceeds the crates.io package limit, so the registry
  `tzf-dist` package ships `lite.tzb` alone. Enabling `full` needs the tzf-dist
  git source, either by taking tzf-rs itself from git or by adding a
  `[patch.crates-io]` entry for `tzf-dist`; see the README. v1 had the same
  constraint; v2 documents it explicitly.
- `full` is mutually exclusive with `bundled`: use
  `default-features = false`. Enabling both is a `compile_error!`.

`.tzb` is the only format tzf-rs consumes. The `.tzm` memory image stays a
Go-runtime optimization (zero-copy ring aliasing); geometry-rs polygons own
their storage, so a Rust `.tzm` loader saved ~3 ms of open time while using
more memory — measured, then dropped. Opening `.tzm` bytes returns
`Error::Profile`.

### Measured

v1 (protobuf) → v2 (`.tzb`), Apple M3 Max, dataset `2026c`, measured
2026-08-28:

| Metric                            | v1 (pb)                          | v2 (.tzb)                     |
| --------------------------------- | -------------------------------- | ----------------------------- |
| `DefaultFinder` open, lite        | 71 ms                            | 18 ms cold / 12.7 ms warm     |
| `DefaultFinder` open, full        | 239 ms                           | 66 ms cold                    |
| Peak RSS, lite / full             | 77.8 / 300 MiB                   | 44.1 / 212.8 MiB              |
| Query, random city / edge city    | 316 / 457 ns                     | 260 / 403 ns                  |
| crates.io data payload            | 6.9 MB topo + 2.0 MB preindex    | 3.97 MB (one file)            |
| `EmbeddedFinder` (new in v2)      | —                                | 2.1 ms open, 5.9 MiB RSS, 1.6–3.9 µs/query |

Current `make bench` medians on the same machine (dataset `2026c`). The
whole-dataset sweep is the stable figure: the single-coordinate benches draw
one random city and reuse it for the whole measurement, so their absolute value
varies with the draw.

| Scenario                                            | DefaultFinder | EmbeddedFinder |
| --------------------------------------------------- | ------------: | -------------: |
| `get_tz_name` over 154,248 cities                   | 25.1 ms (~163 ns/query) | 147.0 ms (~950 ns/query) |
| `get_tz_name`, edge city (preindex miss)            | 434 ns        | 4.03 µs        |
| `get_tz_names`, edge city                           | 531 ns        | 5.71 µs        |
| open (`new()`)                                      | 13.0 ms       | 2.05 ms        |

### Parity

Cross-language and cross-mechanism parity is pinned by tests: `DefaultFinder`
(expanded) and `EmbeddedFinder` (in-place) return identical results, verified
against the Go `tzf/v2` reference over ~195k boundary-heavy samples per
artifact plus the world-cities dataset.
