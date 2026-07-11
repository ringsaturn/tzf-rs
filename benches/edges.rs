use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lazy_static::lazy_static;
use serde::Deserialize;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use tzf_dist::load_topology_compress_topo;
use tzf_rs::{DefaultFinder, Finder, FinderOptions, FuzzyFinder, pbgen};

#[derive(Debug, Deserialize)]
struct EdgeCity {
    lng: f64,
    lat: f64,
}

lazy_static! {
    static ref EDGE_CITIES: Vec<EdgeCity> = {
        let data = fs::read_to_string("benches/edges.json")
            .expect("benches/edges.json not found — run `cargo run --example gen_edges` first");
        serde_json::from_str(&data).expect("invalid benches/edges.json")
    };
    static ref FUZZY_FINDER: FuzzyFinder = FuzzyFinder::default();
    static ref FINDER_NO_INDEX: Finder = {
        let tzs = pbgen::CompressedTopoTimezones::try_from(load_topology_compress_topo())
            .unwrap_or_default();
        Finder::from_compressed_topo_with_options(tzs, FinderOptions::no_index())
    };
    static ref FINDER_INTEGER_RAYCAST: Finder = {
        let tzs = pbgen::CompressedTopoTimezones::try_from(load_topology_compress_topo())
            .unwrap_or_default();
        Finder::from_compressed_topo_with_options(tzs, FinderOptions::no_index_integer_raycast())
    };
    static ref FINDER_YSTRIPES: Finder = {
        let tzs = pbgen::CompressedTopoTimezones::try_from(load_topology_compress_topo())
            .unwrap_or_default();
        Finder::from_compressed_topo_with_options(tzs, FinderOptions::y_stripes())
    };
    static ref DEFAULT_FINDER_NO_INDEX: DefaultFinder =
        DefaultFinder::new_with_options(FinderOptions::no_index());
    static ref DEFAULT_FINDER_YSTRIPES: DefaultFinder =
        DefaultFinder::new_with_options(FinderOptions::y_stripes());
}

static EDGE_IDX: AtomicUsize = AtomicUsize::new(0);

fn next_edge_city() -> &'static EdgeCity {
    let idx = EDGE_IDX.fetch_add(1, Ordering::Relaxed) % EDGE_CITIES.len();
    &EDGE_CITIES[idx]
}

/// FuzzyFinder miss cost vs DefaultFinder full fallback path.
fn bench_fuzzy_vs_fallback(c: &mut Criterion) {
    let mut group = c.benchmark_group("EdgeCities/FuzzyVsFallback");

    let _ = FUZZY_FINDER.get_tz_name(0.0, 0.0);
    let _ = DEFAULT_FINDER_YSTRIPES.get_tz_name(0.0, 0.0);
    let _ = EDGE_CITIES.len();

    let i = &0;
    group.bench_with_input(BenchmarkId::new("FuzzyFinder_miss", i), i, |b, _| {
        b.iter(|| {
            let city = next_edge_city();
            let _ = FUZZY_FINDER.get_tz_name(city.lng, city.lat);
        });
    });
    group.bench_with_input(
        BenchmarkId::new("DefaultFinder_YStripes_fallback", i),
        i,
        |b, _| {
            b.iter(|| {
                let city = next_edge_city();
                let _ = DEFAULT_FINDER_YSTRIPES.get_tz_name(city.lng, city.lat);
            });
        },
    );

    group.finish();
}

/// Finder index modes on edge cities (polygon lookup only, no FuzzyFinder).
fn bench_finder_index_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("EdgeCities/FinderIndexModes");

    let _ = FINDER_NO_INDEX.get_tz_name(0.0, 0.0);
    let _ = FINDER_YSTRIPES.get_tz_name(0.0, 0.0);

    let i = &0;
    group.bench_with_input(BenchmarkId::new("NoIndex", i), i, |b, _| {
        b.iter(|| {
            let city = next_edge_city();
            let _ = FINDER_NO_INDEX.get_tz_name(city.lng, city.lat);
        });
    });
    group.bench_with_input(BenchmarkId::new("IntegerRaycast", i), i, |b, _| {
        b.iter(|| {
            let city = next_edge_city();
            let _ = FINDER_INTEGER_RAYCAST.get_tz_name(city.lng, city.lat);
        });
    });
    group.bench_with_input(BenchmarkId::new("YStripes", i), i, |b, _| {
        b.iter(|| {
            let city = next_edge_city();
            let _ = FINDER_YSTRIPES.get_tz_name(city.lng, city.lat);
        });
    });

    group.finish();
}

/// DefaultFinder index modes on edge cities (FuzzyFinder miss + polygon fallback).
fn bench_default_finder_index_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("EdgeCities/DefaultFinderIndexModes");

    let _ = DEFAULT_FINDER_NO_INDEX.get_tz_name(0.0, 0.0);
    let _ = DEFAULT_FINDER_YSTRIPES.get_tz_name(0.0, 0.0);

    let i = &0;
    group.bench_with_input(BenchmarkId::new("NoIndex", i), i, |b, _| {
        b.iter(|| {
            let city = next_edge_city();
            let _ = DEFAULT_FINDER_NO_INDEX.get_tz_name(city.lng, city.lat);
        });
    });
    group.bench_with_input(BenchmarkId::new("YStripes", i), i, |b, _| {
        b.iter(|| {
            let city = next_edge_city();
            let _ = DEFAULT_FINDER_YSTRIPES.get_tz_name(city.lng, city.lat);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_fuzzy_vs_fallback,
    bench_finder_index_modes,
    bench_default_finder_index_modes
);
criterion_main!(benches);
