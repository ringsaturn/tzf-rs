# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`tzf-rs` is a fast timezone finder library for Rust that converts longitude/latitude coordinates to timezone names. It provides three main finder types:

- **Finder**: Most accurate, uses polygon geometry for precise lookups (~3000ns per query)
- **FuzzyFinder**: Fast preindex-based lookups for most locations (~400ns per query)
- **DefaultFinder**: Combines both, tries FuzzyFinder first, falls back to Finder

The library uses simplified polygon data by default but supports full accuracy data for 100% precise lookups.

## Essential Commands

### Building and Testing
```bash
# Build library only (no CLI binary)
cargo build --no-default-features

# Build with CLI binary (default)
cargo build

# Run tests
cargo test

# Run benchmarks
cargo bench
```

### CLI Usage
```bash
# Single coordinate lookup
cargo run -- --lng 116.3883 --lat 39.9289

# Multiple coordinates from stdin
echo "116.3883 39.9289" | cargo run -- --stdin-order lng-lat
```

### Development Tools
```bash
# Generate license information
make THIRDPARTY.yml

# Install CLI globally
cargo install --path . --no-default-features
```

## Code Architecture

### Core Library Structure (`src/lib.rs`)
- **Item struct**: Internal representation of timezone polygons
- **Finder**: Ray-casting algorithm for precise polygon containment
- **FuzzyFinder**: HashMap-based preindex lookup using map tiles 
- **DefaultFinder**: Hybrid approach combining both finders

### Key Dependencies
- `geometry-rs`: Ray casting polygon containment algorithm
- `tzf-rel`: Binary timezone data (precompiled protobuf)
- `prost`: Protocol buffer serialization

### Data Sources
- Simplified data: Built-in via `tzf-rel` dependency
- Full accuracy data: External 90MB `combined-with-oceans.bin` file

### Performance Strategy
1. Preindex tiles (zoom levels) for fast majority-case lookups
2. Precise polygon geometry for edge cases and 100% accuracy
3. Coordinate shifting for handling simplified polygon gaps

### Testing Approach
- `tests/basic_test.rs`: Core functionality validation
- `benches/finders.rs`: Performance benchmarking using Criterion
- Global city dataset (`cities-json`) for realistic benchmarks

### CLI Binary (`src/bin/tzf.rs`)
Optional CLI tool (requires `clap` feature) supporting:
- Single coordinate queries
- Batch processing from stdin
- Multiple coordinate formats (lng-lat, lat-lng)

## Important Implementation Notes

### Performance Considerations
- Creating Finder instances is expensive - use lazy_static or global variables
- FuzzyFinder uses HashMap for O(1) tile lookups
- Coordinate shifting (±0.01, ±0.02 degrees) handles simplified polygon gaps

### Data Handling
- Default: Uses reduced/simplified polygon data from `tzf-rel`
- Full accuracy: Load external protobuf data via `Finder::from_pb()`
- All coordinates are longitude-latitude pairs (x, y format)

### Key Functions
- `deg2num()`: Converts lat/lng to map tile coordinates
- `get_tz_name()`: Returns single timezone name
- `get_tz_names()`: Returns all matching timezones (for overlapping areas)