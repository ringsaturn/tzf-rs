use cities_json::get_random_cities;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lazy_static::lazy_static;
use tzf_rs::{DefaultFinder, FuzzyFinder};

lazy_static! {
    static ref FULL_FINDER: DefaultFinder = DefaultFinder::new_full();
}

fn bench_full_finder_random_city() {
    let city = get_random_cities();
    let _ = FULL_FINDER.get_tz_name(city.lng, city.lat);
}

fn bench_full_finders(c: &mut Criterion) {
    let mut group = c.benchmark_group("FullFinders");

    let _ = FULL_FINDER.get_tz_name(116.3883, 39.9289);

    let i = &0;
    group.bench_with_input(BenchmarkId::new("DefaultFinder_Full", i), i, |b, _i| {
        b.iter(|| bench_full_finder_random_city());
    });

    group.finish();
}

criterion_group!(benches, bench_full_finders);
criterion_main!(benches);
