# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`tzf-rs` is a fast timezone finder library for Rust that converts longitude/latitude coordinates to timezone names. Since v2 it is protobuf-free: the data source is the TZF embedded binary format (`.tzb`) shipped by the `tzf-dist` crate, and the public surface is two finder types:

- **DefaultFinder**: Recommended. Expands `.tzb` geometry into geometry-rs polygons at load (~13 ms); `get_tz_name` answers from the FUZZY preindex tiles with exact point-in-polygon fallback; `get_tz_names` is always polygon-exact and lexicographically sorted.
- **EmbeddedFinder**: Low-memory. Queries the `.tzb` bytes in place (~4 MB total, ~2 ms open); identical results, microsecond queries.

The Go reference implementation of the format is `github.com/ringsaturn/tzf/v2/internal/embedbin`.

MSRV is **1.88** (edition 2024 + let-chains), declared as `rust-version` in `Cargo.toml`.

## Essential Commands

```bash
# Build library only (no CLI binary)
cargo build --no-default-features --features bundled

# Build with CLI binary (default)
cargo build

# Run tests (release recommended; parity sweeps are slow in debug)
cargo test --release --features export-geojson

# Lint both data legs (see below: --all-features never works here)
make lint

# Run benchmarks
cargo bench

# Single coordinate lookup
cargo run -- --lng 116.3883 --lat 39.9289
```

**`--all-features` never builds this crate.** `bundled` and `full` are mutually
exclusive (`compile_error!` in `src/lib.rs`), and while the dev-form path
dependency is active cargo additionally refuses "depends on crate `tzf-dist`
multiple times with different names". Lint and test the two legs separately;
`make lint` and `make ci` already do.

## Development data setup

Until tzf-dist publishes the `.tzb` artifact release, Cargo.toml carries a path dependency on a sibling `../tzf-dist` checkout (branch `v2-artifacts`). Fill it with real artifacts by running `scripts/build-tzf-dist-dev.sh` in a sibling `tzf` checkout (branch `embedded`), which builds the artifact set from upstream raw GeoJSON and installs it over tzf-dist's committed placeholders.

Cargo.toml documents the publishable **release form** of those dependencies
next to the dev form; `release-runbook-v2.md` is the ordered switch-and-tag
procedure. `.github/workflows/rust.yml` and `deploy_doc.yml` detect which form
is active and skip the Go bootstrap automatically once the release form lands.

## Code Architecture

- `src/tzb/` — the `.tzb` reader, mirroring Go `internal/embedbin`:
  - `mod.rs`: constants, `Error`, BBox, CRC32 (slicing-by-8), zigzag-LEB128 cursor
  - `reader.rs`: open/validation, directory records, GRID candidates, in-place PIP query walk (spec §8)
  - `fuzzy.rs`: FUZZY section validation + in-place binary-search lookup
  - `expand.rs`: expansion loader (E profile → open rings), per-timezone expansion for GeoJSON
  - `raycast.rs`: segment raycast (port of geometry-rs/tidwall semantics; geometry-rs 0.5 keeps its raycast result fields private)
  - `tile.rs`: slippy-map tile math, bit-identical to Go `geom.TileID`
- `src/finder.rs` — materialized polygon finder (geometry-rs `I32Polygon` + YStripes, always on), dense GRID index, FUZZY hash-map fast path, parallel item assembly
- `src/lib.rs` — public API (`DefaultFinder`, `EmbeddedFinder`, `Error`, `deg2num`); both finders expose `get_tz_name`, `get_tz_names`, `timezonenames`, `data_version`, and under `export-geojson` `to_geojson`, `get_tz_geojson`, `to_preindex_geojson`, `get_tz_preindex_geojson`
- `src/geojson.rs` — GeoJSON export types (feature `export-geojson`)

`src/lib.rs` also carries a `#[cfg(doctest)] ReadmeDoctests` item that runs
every Rust-tagged fenced block in README.md as a doctest under
`bundled,export-geojson`, so README samples cannot drift from the API.

## Correctness invariants

- Rings from the loaders are **open**; geometry-rs stores rings **closed** — `close_ring` in `src/finder.rs` appends the closing vertex. Never feed open rings to `I32Polygon`.
- Boundary semantics: exterior rings allow on-edge containment, hole rings do not (`contains_point_allow_on_edge`); a border query belongs to every touching polygon.
- `tests/parity_test.rs` pins DefaultFinder ≡ EmbeddedFinder; cross-language parity with the Go v2 reference was verified over ~195k samples per artifact.
- tzf-rs consumes the `.tzb` (E) profile only; `.tzm` memory images are rejected with `Error::Profile`. The `.tzm` format serves the Go runtime's zero-copy ring aliasing, which geometry-rs's owned polygon storage cannot exploit (measured: ~3 ms faster open, more memory — dropped 2026-08-28).

## Features

- `bundled` (default): lite `.tzb` via crates.io tzf-dist. Mutually exclusive with the `full` feature.
- `export-geojson`: GeoJSON export methods.
- `clap` (default): the `tzf` CLI binary.
- `full`: full-precision `.tzb` (~14 MB) and `DefaultFinder::new_full()`. **The
  data is git-only** — `full.tzb` exceeds the crates.io package limit, so the
  registry `tzf-dist` package excludes it *and* `src/full.rs`. Enabling `full`
  against the registry copy does not compile. Consumers need the tzf-dist git
  source: either take tzf-rs itself from git, or add `[patch.crates-io]` for
  `tzf-dist`. Both recipes are in README's "Setup 100% Accurate Lookup".
  Inside this repo `full` always works, because the manifest's own
  `tzf-dist-git` entry keeps the git (or path) source.
