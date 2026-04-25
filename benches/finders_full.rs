use cities_json::get_random_cities;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use lazy_static::lazy_static;
use tzf_dist_git::load_compress_topo;
use tzf_rs::{DefaultFinder, Finder, FinderOptions, FuzzyFinder, pbgen};

lazy_static! {
    static ref FULL_FINDER: DefaultFinder = DefaultFinder::new_full();
    static ref FULL_FINDER_NO_INDEX: DefaultFinder =
        DefaultFinder::new_full_with_options(FinderOptions::no_index());
    static ref FINDER_FULL_YSTRIPES: Finder = {
        let tzs =
            pbgen::CompressedTopoTimezones::try_from(load_compress_topo()).unwrap_or_default();
        Finder::from_compressed_topo_with_options(tzs, FinderOptions::y_stripes())
    };
    static ref FINDER_FULL_NO_INDEX: Finder = {
        let tzs =
            pbgen::CompressedTopoTimezones::try_from(load_compress_topo()).unwrap_or_default();
        Finder::from_compressed_topo_with_options(tzs, FinderOptions::no_index())
    };
    static ref FUZZY_FINDER: FuzzyFinder = FuzzyFinder::default();
}

fn bench_full_finders(c: &mut Criterion) {
    let mut group = c.benchmark_group("FullFinders");

    let _ = FULL_FINDER.get_tz_name(116.3883, 39.9289);
    let _ = FINDER_FULL_NO_INDEX.get_tz_name(116.3883, 39.9289);

    let i = &0;
    group.bench_with_input(BenchmarkId::new("DefaultFinder_Full", i), i, |b, _i| {
        b.iter(|| {
            let city = get_random_cities();
            let _ = FULL_FINDER.get_tz_name(city.lng, city.lat);
        });
    });
    group.bench_with_input(BenchmarkId::new("Finder_Full_NoIndex", i), i, |b, _i| {
        b.iter(|| {
            let city = get_random_cities();
            let _ = FINDER_FULL_NO_INDEX.get_tz_name(city.lng, city.lat);
        });
    });
    group.bench_with_input(BenchmarkId::new("FuzzyFinder", i), i, |b, _i| {
        b.iter(|| {
            let city = get_random_cities();
            let _ = FUZZY_FINDER.get_tz_name(city.lng, city.lat);
        });
    });

    group.finish();
}

fn bench_finder_full_index_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("FullFinderIndexModes");

    let _ = FINDER_FULL_YSTRIPES.get_tz_name(116.3883, 39.9289);
    let _ = FINDER_FULL_NO_INDEX.get_tz_name(116.3883, 39.9289);

    let i = &0;
    group.bench_with_input(BenchmarkId::new("YStripesOnly", i), i, |b, _i| {
        b.iter(|| {
            let city = get_random_cities();
            let _ = FINDER_FULL_YSTRIPES.get_tz_name(city.lng, city.lat);
        });
    });
    group.bench_with_input(BenchmarkId::new("NoIndex", i), i, |b, _i| {
        b.iter(|| {
            let city = get_random_cities();
            let _ = FINDER_FULL_NO_INDEX.get_tz_name(city.lng, city.lat);
        });
    });

    group.finish();
}

fn bench_default_finder_full_index_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("FullDefaultFinderIndexModes");

    let _ = FULL_FINDER.get_tz_name(116.3883, 39.9289);
    let _ = FULL_FINDER_NO_INDEX.get_tz_name(116.3883, 39.9289);

    let i = &0;
    group.bench_with_input(BenchmarkId::new("YStripesOnly", i), i, |b, _i| {
        b.iter(|| {
            let city = get_random_cities();
            let _ = FULL_FINDER.get_tz_name(city.lng, city.lat);
        });
    });
    group.bench_with_input(BenchmarkId::new("NoIndex", i), i, |b, _i| {
        b.iter(|| {
            let city = get_random_cities();
            let _ = FULL_FINDER_NO_INDEX.get_tz_name(city.lng, city.lat);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_full_finders,
    bench_finder_full_index_modes,
    bench_default_finder_full_index_modes
);
criterion_main!(benches);
