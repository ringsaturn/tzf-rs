use cities_json::{CITIES, get_random_cities};
use criterion::{Criterion, criterion_group, criterion_main};
use lazy_static::lazy_static;
use std::hint::black_box;
use tzf_rs::{DefaultFinder, EmbeddedFinder};

lazy_static! {
    static ref DEFAULT_FINDER: DefaultFinder = DefaultFinder::new();
    static ref EMBEDDED_FINDER: EmbeddedFinder = EmbeddedFinder::new();
}

fn bench_queries(c: &mut Criterion) {
    let city = get_random_cities();

    c.bench_function("default_finder_random_city", |b| {
        b.iter(|| black_box(DEFAULT_FINDER.get_tz_name(city.lng, city.lat)));
    });
    c.bench_function("embedded_finder_random_city", |b| {
        b.iter(|| black_box(EMBEDDED_FINDER.get_tz_name(city.lng, city.lat)));
    });
    c.bench_function("default_finder_get_tz_names_random_city", |b| {
        b.iter(|| black_box(DEFAULT_FINDER.get_tz_names(city.lng, city.lat)));
    });
    c.bench_function("embedded_finder_get_tz_names_random_city", |b| {
        b.iter(|| black_box(EMBEDDED_FINDER.get_tz_names(city.lng, city.lat)));
    });
}

/// Whole-dataset sweep: amortizes per-query variance across every city.
fn bench_all_cities(c: &mut Criterion) {
    let mut group = c.benchmark_group("all_cities");
    group.sample_size(10);
    group.bench_function("default_finder", |b| {
        b.iter(|| {
            for city in CITIES.iter() {
                black_box(DEFAULT_FINDER.get_tz_name(city.lng, city.lat));
            }
        });
    });
    group.bench_function("embedded_finder", |b| {
        b.iter(|| {
            for city in CITIES.iter() {
                black_box(EMBEDDED_FINDER.get_tz_name(city.lng, city.lat));
            }
        });
    });
    group.finish();
}

fn bench_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("load");
    group.sample_size(10);
    group.bench_function("default_finder_new", |b| {
        b.iter(|| black_box(DefaultFinder::new()));
    });
    group.bench_function("embedded_finder_new", |b| {
        b.iter(|| black_box(EmbeddedFinder::new()));
    });
    group.finish();
}

criterion_group!(benches, bench_queries, bench_all_cities, bench_load);
criterion_main!(benches);
