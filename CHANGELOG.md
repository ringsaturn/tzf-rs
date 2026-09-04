# Changelog

## v2.0.0 (unreleased)

tzf-rs v2 is protobuf-free. The data source is the TZF embedded binary format
(`.tzb`) shipped by
[tzf-dist](https://github.com/ringsaturn/tzf-dist); the protobuf artifacts of
the v1 line are no longer published, so staying on v1 means staying on its
last data release.

### Public surface

Two finder types, mirroring the Go `tzf/v2` design:

- `DefaultFinder` — the recommended finder. Loads a `.tzb` by expanding its
  geometry into geometry-rs polygons (`new()`, `new_full()`, `from_tzb`).
  `get_tz_name` answers from the FUZZY preindex tiles first and falls back
  to exact point-in-polygon; `get_tz_names` stays polygon-exact.
- `EmbeddedFinder` — the low-memory finder: queries `.tzb` bytes in place
  (~4 MB total for the bundled lite data), FUZZY-first with the
  compressed-geometry scan as fallback. Accepts `&'static [u8]` or `Vec<u8>`.

### Removed

- Every protobuf-typed API: `Finder::from_pb`, `Finder::from_compressed_topo`,
  `FuzzyFinder::from_pb`, the `pbgen` module, the `prost` dependency and the
  protobuf build machinery.
- The `Finder` and `FuzzyFinder` types. `DefaultFinder` covers `Finder`
  (`get_tz_names` is polygon-exact); tile-only lookup is no longer a public
  mechanism in any form — the preindex is the internal fast path inside every
  finder.
- `FinderOptions` and the `*_with_options` constructors — the YStripes index
  is always enabled.
- `FuzzyFinder`'s tile-bbox GeoJSON export, with no replacement.
- `revert_timezones` (it took a protobuf type).

### Migration

| v1                                   | v2                                       |
| ------------------------------------ | ---------------------------------------- |
| `DefaultFinder::new()`               | unchanged                                |
| `DefaultFinder::new_full()`          | unchanged (`full` feature, git-only)     |
| `Finder::new()`                      | `DefaultFinder::new()`                   |
| `FuzzyFinder::new()`                 | `DefaultFinder::new()`                   |
| `Finder::from_compressed_topo(pb)`   | `DefaultFinder::from_tzb(bytes)`         |
| `FinderOptions` / `new_with_options` | removed                                  |
| `finder.finder.get_tz_geojson(...)`  | `finder.get_tz_geojson(...)`             |

### Behavior changes

- `get_tz_names` results are sorted lexicographically (matching the Go v2
  finder).
- GeoJSON exports omit the duplicated junction vertices the protobuf
  expansion retained (zero-length segments; query results are unaffected).
- Malformed data now surfaces as `Err(tzf_rs::Error)` from the byte
  constructors instead of an empty default finder: files are CRC-checked and
  structurally validated at open.

### Data features

- `bundled` (default): lite `.tzb` from tzf-dist on crates.io (~4 MB).
- `full` (git-only): full-precision `.tzb` (~14 MB), enables `new_full()`.

`.tzb` is the only format tzf-rs consumes. The `.tzm` memory image stays a
Go-runtime optimization (zero-copy ring aliasing); geometry-rs polygons own
their storage, so a Rust `.tzm` loader saved ~3 ms of open time while using
more memory — measured, then dropped. Opening `.tzm` bytes returns
`Error::Profile`.

### Parity

Cross-language and cross-mechanism parity is pinned by tests: `DefaultFinder`
(expanded) and `EmbeddedFinder` (in-place) return identical results, verified
against the Go `tzf/v2` reference over ~195k boundary-heavy samples per
artifact plus the world-cities dataset.
