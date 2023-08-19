use cities_json::CITIES;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use tzf_rs::{DefaultFinder, Finder};

lazy_static! {
    static ref DEFAULT_FINDER: DefaultFinder = DefaultFinder::default();
    static ref FINDER: Finder = Finder::default();
}

fn bench_default_finder_random_city(_i: usize) {
    let city = CITIES.choose(&mut rand::thread_rng()).unwrap();
    let _ = DEFAULT_FINDER.get_tz_name(city.lng, city.lat);
}

fn bench_finder_random_city(_i: usize) {
    let city = CITIES.choose(&mut rand::thread_rng()).unwrap();
    let _ = FINDER.get_tz_name(city.lng, city.lat);
}

fn bench_finders(c: &mut Criterion) {
    let mut group = c.benchmark_group("Finders");

    // warmup
    let _ = DEFAULT_FINDER.get_tz_name(116.3883, 39.9289);
    let _ = FINDER.get_tz_name(116.3883, 39.9289);

    let i = &0;
    group.bench_with_input(BenchmarkId::new("Default", i), i, |b, _i| {
        b.iter(|| bench_default_finder_random_city(black_box(1)));
    });
    group.bench_with_input(BenchmarkId::new("Finder", i), i, |b, _i| {
        b.iter(|| bench_finder_random_city(black_box(1)));
    });

    group.finish();
}

criterion_group!(benches, bench_finders);
criterion_main!(benches);
