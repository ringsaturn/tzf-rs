use cities_json::{CITIES, get_random_cities};
use criterion::{Criterion, criterion_group, criterion_main};
use lazy_static::lazy_static;
use std::hint::black_box;
use tzf_rs::DefaultFinder;

lazy_static! {
    static ref FULL_FINDER: DefaultFinder = DefaultFinder::new_full();
}

fn bench_full(c: &mut Criterion) {
    let city = get_random_cities();

    c.bench_function("full_finder_random_city", |b| {
        b.iter(|| black_box(FULL_FINDER.get_tz_name(city.lng, city.lat)));
    });
    c.bench_function("full_finder_get_tz_names_random_city", |b| {
        b.iter(|| black_box(FULL_FINDER.get_tz_names(city.lng, city.lat)));
    });

    let mut group = c.benchmark_group("all_cities");
    group.sample_size(10);
    group.bench_function("full_finder", |b| {
        b.iter(|| {
            for city in CITIES.iter() {
                black_box(FULL_FINDER.get_tz_name(city.lng, city.lat));
            }
        });
    });
    group.finish();
}

criterion_group!(benches, bench_full);
criterion_main!(benches);
