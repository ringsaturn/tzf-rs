# AGENTS.md

This file provides guidance to agentic coding tools when working with code in this repository.

## Project Overview

`tzf-rs` is a fast timezone finder library for Rust that converts longitude/latitude coordinates to timezone names. Since v2 it is protobuf-free: the data source is the TZF embedded binary format (`.tzb`) shipped by the `tzf-dist` crate, and the public surface is two finder types:

- **DefaultFinder**: Recommended. Expands `.tzb` geometry into geometry-rs polygons at load (~13 ms); `get_tz_name` answers from the FUZZY preindex tiles (~100 ns) with exact point-in-polygon fallback (~300-600 ns); `get_tz_names` is always polygon-exact and lexicographically sorted.
- **EmbeddedFinder**: Low-memory. Queries the `.tzb` bytes in place (~4 MB total, ~2 ms open); identical results, microsecond queries.

The Go reference implementation of the format is `github.com/ringsaturn/tzf/v2/internal/embedbin`.

## Essential Commands

```bash
# Build library only (no CLI binary)
cargo build --no-default-features --features bundled

# Build with CLI binary (default)
cargo build

# Run tests (release recommended; parity sweeps are slow in debug)
cargo test --release --features export-geojson

# Run benchmarks
cargo bench

# Single coordinate lookup
cargo run -- --lng 116.3883 --lat 39.9289
```

## Development data setup

Until tzf-dist publishes the `.tzb` artifact release, Cargo.toml carries a path dependency on a sibling `../tzf-dist` checkout (branch `v2-artifacts`). Fill it with real artifacts by running `scripts/build-tzf-dist-dev.sh` in a sibling `tzf` checkout (branch `embedded`), which builds the artifact set from upstream raw GeoJSON and installs it over tzf-dist's committed placeholders.

## Code Architecture

- `src/tzb/` — the `.tzb` reader, mirroring Go `internal/embedbin`:
  - `mod.rs`: constants, `Error`, BBox, CRC32 (slicing-by-8), zigzag-LEB128 cursor
  - `reader.rs`: open/validation, directory records, GRID candidates, in-place PIP query walk (spec §8)
  - `fuzzy.rs`: FUZZY section validation + in-place binary-search lookup
  - `expand.rs`: expansion loader (E profile → open rings), per-timezone expansion for GeoJSON
  - `raycast.rs`: segment raycast (port of geometry-rs/tidwall semantics; geometry-rs 0.5 keeps its raycast result fields private)
  - `tile.rs`: slippy-map tile math, bit-identical to Go `geom.TileID`
- `src/finder.rs` — materialized polygon finder (geometry-rs `I32Polygon` + YStripes, always on), dense GRID index, FUZZY hash-map fast path, parallel item assembly
- `src/lib.rs` — public API (`DefaultFinder`, `EmbeddedFinder`, `Error`, `deg2num`)
- `src/geojson.rs` — GeoJSON export types (feature `export-geojson`)

## Correctness invariants

- Rings from the loaders are **open**; geometry-rs stores rings **closed** — `close_ring` in `src/finder.rs` appends the closing vertex. Never feed open rings to `I32Polygon`.
- Boundary semantics: exterior rings allow on-edge containment, hole rings do not (`contains_point_allow_on_edge`); a border query belongs to every touching polygon.
- `tests/parity_test.rs` pins DefaultFinder ≡ EmbeddedFinder; cross-language parity with the Go v2 reference was verified over ~195k samples per artifact.
- tzf-rs consumes the `.tzb` (E) profile only; `.tzm` memory images are rejected with `Error::Profile`. The `.tzm` format serves the Go runtime's zero-copy ring aliasing, which geometry-rs's owned polygon storage cannot exploit (measured: ~3 ms faster open, more memory — dropped 2026-08-28).

## Features

- `bundled` (default): lite `.tzb` via crates.io tzf-dist. Mutually exclusive with the git-only `full` (full-precision `.tzb`) feature.
- `export-geojson`: GeoJSON export methods.
- `clap`: the `tzf` CLI binary.
