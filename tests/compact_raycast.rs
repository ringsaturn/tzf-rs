use serde::Deserialize;
use std::fs;
#[cfg(feature = "bundled")]
use tzf_dist::load_topology_compress_topo;
#[cfg(feature = "full")]
use tzf_dist_git::load_topology_compress_topo;
use tzf_rs::{Finder, FinderOptions, pbgen};

#[derive(Deserialize)]
struct Coordinate {
    lng: f64,
    lat: f64,
}

fn load() -> pbgen::CompressedTopoTimezones {
    pbgen::CompressedTopoTimezones::try_from(load_topology_compress_topo()).unwrap_or_default()
}

fn edge_coordinates() -> Vec<Coordinate> {
    serde_json::from_str(
        &fs::read_to_string("benches/edges.json").expect("missing edge benchmark data"),
    )
    .expect("invalid edge benchmark data")
}

#[test]
fn integer_and_float_raycasts_match_on_edge_cities() {
    let float =
        Finder::from_compressed_topo_with_options(load(), FinderOptions::no_index_float_raycast());
    let integer = Finder::from_compressed_topo_with_options(
        load(),
        FinderOptions::no_index_integer_raycast(),
    );
    for coordinate in edge_coordinates() {
        assert_eq!(
            float.get_tz_names(coordinate.lng, coordinate.lat),
            integer.get_tz_names(coordinate.lng, coordinate.lat),
            "raycast mismatch at {}, {}",
            coordinate.lng,
            coordinate.lat,
        );
    }
}

#[test]
fn y_stripes_and_linear_scan_match_on_edge_cities() {
    let indexed = Finder::from_compressed_topo_with_options(load(), FinderOptions::y_stripes());
    let linear =
        Finder::from_compressed_topo_with_options(load(), FinderOptions::no_index_float_raycast());
    for coordinate in edge_coordinates() {
        assert_eq!(
            indexed.get_tz_names(coordinate.lng, coordinate.lat),
            linear.get_tz_names(coordinate.lng, coordinate.lat),
            "index mismatch at {}, {}",
            coordinate.lng,
            coordinate.lat,
        );
    }
}

// Deterministic splitmix64 so the cross-check needs no rand dependency.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn uniform(&mut self, min: f64, max: f64) -> f64 {
        let unit = (self.next() >> 11) as f64 / (1u64 << 53) as f64;
        min + unit * (max - min)
    }
}

fn cross_check_y_stripes_vs_linear(random_points: usize, grid_points: usize) {
    let indexed = Finder::from_compressed_topo_with_options(load(), FinderOptions::y_stripes());
    let linear =
        Finder::from_compressed_topo_with_options(load(), FinderOptions::no_index_float_raycast());
    let mut rng = SplitMix64(0x7A5F_2026_0711_0001);
    for i in 0..random_points {
        let lng = rng.uniform(-180.0, 180.0);
        let lat = rng.uniform(-90.0, 90.0);
        assert_eq!(
            indexed.get_tz_names(lng, lat),
            linear.get_tz_names(lng, lat),
            "index mismatch at random point #{i}: {lng}, {lat}",
        );
    }
    // Points snapped to the 1e-5 grid land exactly on stored vertices and
    // edges, the worst case for representation differences.
    for i in 0..grid_points {
        let lng = f64::from((rng.uniform(-180.0, 180.0) * 1e5).round() as i32) / 1e5;
        let lat = f64::from((rng.uniform(-90.0, 90.0) * 1e5).round() as i32) / 1e5;
        assert_eq!(
            indexed.get_tz_names(lng, lat),
            linear.get_tz_names(lng, lat),
            "index mismatch at grid point #{i}: {lng}, {lat}",
        );
    }
}

#[test]
fn y_stripes_and_linear_scan_match_on_random_and_grid_points() {
    cross_check_y_stripes_vs_linear(20_000, 10_000);
}

/// Full-size cross-check matching the Go-side methodology (§5 of the port
/// plan). Run with: `cargo test --release -- --ignored`
#[test]
#[ignore = "expensive; run with --release -- --ignored"]
fn y_stripes_and_linear_scan_full_cross_check() {
    cross_check_y_stripes_vs_linear(500_000, 200_000);
}

/// Guards the 1e5 equivalence argument: every polyline-decoded integer must
/// survive the degree → storage-space round trip exactly, i.e.
/// `round(f64::from(k) / 1e5 * 1e5) == k`. If the scale ever changed to a
/// value where this fails, the "index and query both live in storage space"
/// design would silently lose its bit-exactness guarantee.
#[test]
fn scale_round_trip_is_exact_on_1e5_grid() {
    let mut rng = SplitMix64(0x7A5F_2026_0711_0002);
    let check = |k: i32| {
        let deg = f64::from(k) / 1e5;
        assert_eq!(
            (deg * 1e5).round() as i64,
            i64::from(k),
            "round trip failed for {k}"
        );
    };
    for bound in [-18_000_000, -9_000_000, -1, 0, 1, 9_000_000, 18_000_000] {
        check(bound);
    }
    for _ in 0..1_000_000 {
        let k = (rng.next() % 36_000_001) as i64 - 18_000_000;
        check(k as i32);
    }
}
