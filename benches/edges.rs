//! Edge-city benchmarks: cities where the FUZZY preindex misses, so every
//! query pays the full polygon path — the worst case for both mechanisms.

use criterion::{Criterion, criterion_group, criterion_main};
use lazy_static::lazy_static;
use serde::Deserialize;
use std::fs;
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use tzf_rs::{DefaultFinder, EmbeddedFinder};

#[derive(Debug, Deserialize)]
struct EdgeCity {
    lng: f64,
    lat: f64,
}

lazy_static! {
    static ref EDGE_CITIES: Vec<EdgeCity> = {
        let data = fs::read_to_string("benches/edges.json").expect("benches/edges.json not found");
        serde_json::from_str(&data).expect("invalid benches/edges.json")
    };
    static ref DEFAULT_FINDER: DefaultFinder = DefaultFinder::new();
    static ref EMBEDDED_FINDER: EmbeddedFinder = EmbeddedFinder::new();
}

static EDGE_IDX: AtomicUsize = AtomicUsize::new(0);

fn next_edge_city() -> &'static EdgeCity {
    let idx = EDGE_IDX.fetch_add(1, Ordering::Relaxed) % EDGE_CITIES.len();
    &EDGE_CITIES[idx]
}

fn bench_edge_cities(c: &mut Criterion) {
    let _ = EDGE_CITIES.len();
    let _ = DEFAULT_FINDER.get_tz_name(0.0, 0.0);
    let _ = EMBEDDED_FINDER.get_tz_name(0.0, 0.0);

    let mut group = c.benchmark_group("EdgeCities");
    group.bench_function("default_finder", |b| {
        b.iter(|| {
            let city = next_edge_city();
            black_box(DEFAULT_FINDER.get_tz_name(city.lng, city.lat));
        });
    });
    group.bench_function("embedded_finder", |b| {
        b.iter(|| {
            let city = next_edge_city();
            black_box(EMBEDDED_FINDER.get_tz_name(city.lng, city.lat));
        });
    });
    group.bench_function("default_finder_get_tz_names", |b| {
        b.iter(|| {
            let city = next_edge_city();
            black_box(DEFAULT_FINDER.get_tz_names(city.lng, city.lat));
        });
    });
    group.bench_function("embedded_finder_get_tz_names", |b| {
        b.iter(|| {
            let city = next_edge_city();
            black_box(EMBEDDED_FINDER.get_tz_names(city.lng, city.lat));
        });
    });
    group.finish();
}

criterion_group!(benches, bench_edge_cities);
criterion_main!(benches);
