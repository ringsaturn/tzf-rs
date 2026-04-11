use cities_json::get_random_cities;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lazy_static::lazy_static;
use tzf_rel::load_reduced;
use tzf_rs::{DefaultFinder, Finder, FinderOptions, FuzzyFinder, pbgen};

lazy_static! {
    static ref DEFAULT_FINDER: DefaultFinder = DefaultFinder::default();
    static ref FINDER: Finder = Finder::default();
    static ref FUZZY_FINDER: FuzzyFinder = FuzzyFinder::default();
    static ref FINDER_RTREE_ONLY: Finder = {
        let tzs = pbgen::Timezones::try_from(load_reduced()).unwrap_or_default();
        Finder::from_pb_with_options(tzs, FinderOptions::rtree())
    };
    static ref FINDER_QUAD_ONLY: Finder = {
        let tzs = pbgen::Timezones::try_from(load_reduced()).unwrap_or_default();
        Finder::from_pb_with_options(tzs, FinderOptions::quad_tree())
    };
    static ref FINDER_NO_INDEX: Finder = {
        let tzs = pbgen::Timezones::try_from(load_reduced()).unwrap_or_default();
        Finder::from_pb_with_options(tzs, FinderOptions::no_index())
    };
    static ref DEFAULT_FINDER_RTREE_ONLY: DefaultFinder =
        DefaultFinder::new_with_options(FinderOptions::rtree());
    static ref DEFAULT_FINDER_QUAD_ONLY: DefaultFinder =
        DefaultFinder::new_with_options(FinderOptions::quad_tree());
    static ref DEFAULT_FINDER_NO_INDEX: DefaultFinder =
        DefaultFinder::new_with_options(FinderOptions::no_index());
}

fn bench_default_finder_random_city() {
    let city = get_random_cities();
    let _ = DEFAULT_FINDER.get_tz_name(city.lng, city.lat);
}

fn bench_finder_random_city() {
    let city = get_random_cities();
    let _ = FINDER.get_tz_name(city.lng, city.lat);
}

fn bench_finder_rtree_only_random_city() {
    let city = get_random_cities();
    let _ = FINDER_RTREE_ONLY.get_tz_name(city.lng, city.lat);
}

fn bench_finder_quad_only_random_city() {
    let city = get_random_cities();
    let _ = FINDER_QUAD_ONLY.get_tz_name(city.lng, city.lat);
}

fn bench_finder_no_index_random_city() {
    let city = get_random_cities();
    let _ = FINDER_NO_INDEX.get_tz_name(city.lng, city.lat);
}

fn bench_default_finder_rtree_only_random_city() {
    let city = get_random_cities();
    let _ = DEFAULT_FINDER_RTREE_ONLY.get_tz_name(city.lng, city.lat);
}

fn bench_default_finder_quad_only_random_city() {
    let city = get_random_cities();
    let _ = DEFAULT_FINDER_QUAD_ONLY.get_tz_name(city.lng, city.lat);
}

fn bench_default_finder_no_index_random_city() {
    let city = get_random_cities();
    let _ = DEFAULT_FINDER_NO_INDEX.get_tz_name(city.lng, city.lat);
}

fn bench_fuzzy_finder_random_city() {
    let city = get_random_cities();
    let _ = FUZZY_FINDER.get_tz_name(city.lng, city.lat);
}

fn bench_finders(c: &mut Criterion) {
    let mut group = c.benchmark_group("Finders");

    // warmup
    let _ = DEFAULT_FINDER.get_tz_name(116.3883, 39.9289);
    let _ = FINDER.get_tz_name(116.3883, 39.9289);

    let i = &0;
    group.bench_with_input(BenchmarkId::new("DefaultFinder", i), i, |b, _i| {
        b.iter(|| bench_default_finder_random_city());
    });
    group.bench_with_input(BenchmarkId::new("Finder_NoIndex", i), i, |b, _i| {
        b.iter(|| bench_finder_random_city());
    });
    group.bench_with_input(BenchmarkId::new("FuzzyFinder", i), i, |b, _i| {
        b.iter(|| bench_fuzzy_finder_random_city());
    });

    group.finish();
}

fn bench_finder_index_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("FinderIndexModes");

    let _ = FINDER_RTREE_ONLY.get_tz_name(116.3883, 39.9289);
    let _ = FINDER_QUAD_ONLY.get_tz_name(116.3883, 39.9289);
    let _ = FINDER_NO_INDEX.get_tz_name(116.3883, 39.9289);

    let i = &0;
    group.bench_with_input(BenchmarkId::new("RTreeOnly", i), i, |b, _i| {
        b.iter(|| bench_finder_rtree_only_random_city());
    });
    group.bench_with_input(BenchmarkId::new("QuadOnly", i), i, |b, _i| {
        b.iter(|| bench_finder_quad_only_random_city());
    });
    group.bench_with_input(BenchmarkId::new("NoIndex", i), i, |b, _i| {
        b.iter(|| bench_finder_no_index_random_city());
    });

    group.finish();
}

fn bench_default_finder_index_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("DefaultFinderIndexModes");

    let _ = DEFAULT_FINDER_RTREE_ONLY.get_tz_name(116.3883, 39.9289);
    let _ = DEFAULT_FINDER_QUAD_ONLY.get_tz_name(116.3883, 39.9289);
    let _ = DEFAULT_FINDER_NO_INDEX.get_tz_name(116.3883, 39.9289);

    let i = &0;
    group.bench_with_input(BenchmarkId::new("RTreeOnly", i), i, |b, _i| {
        b.iter(|| bench_default_finder_rtree_only_random_city());
    });
    group.bench_with_input(BenchmarkId::new("QuadOnly", i), i, |b, _i| {
        b.iter(|| bench_default_finder_quad_only_random_city());
    });
    group.bench_with_input(BenchmarkId::new("NoIndex", i), i, |b, _i| {
        b.iter(|| bench_default_finder_no_index_random_city());
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_finders,
    bench_finder_index_modes,
    bench_default_finder_index_modes
);
criterion_main!(benches);
