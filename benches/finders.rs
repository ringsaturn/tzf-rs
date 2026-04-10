use cities_json::get_random_cities;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use geometry_rs::PolygonBuildOptions;
use lazy_static::lazy_static;
use tzf_rel::load_reduced;
use tzf_rs::{DefaultFinder, Finder, FuzzyFinder, pbgen};

lazy_static! {
    static ref DEFAULT_FINDER: DefaultFinder = DefaultFinder::default();
    static ref FINDER: Finder = Finder::default();
    static ref FINDER_RTREE_ONLY: Finder = {
        let tzs = pbgen::Timezones::try_from(load_reduced()).unwrap_or_default();
        Finder::from_pb_with_index_options(
            tzs,
            PolygonBuildOptions {
                enable_rtree: true,
                enable_compressed_quad: false,
                rtree_min_segments: 64,
            },
        )
    };
    static ref FINDER_QUAD_ONLY: Finder = {
        let tzs = pbgen::Timezones::try_from(load_reduced()).unwrap_or_default();
        Finder::from_pb_with_index_options(
            tzs,
            PolygonBuildOptions {
                enable_rtree: false,
                enable_compressed_quad: true,
                rtree_min_segments: 64,
            },
        )
    };
    static ref FINDER_NO_INDEX: Finder = {
        let tzs = pbgen::Timezones::try_from(load_reduced()).unwrap_or_default();
        Finder::from_pb_with_index_options(
            tzs,
            PolygonBuildOptions {
                enable_rtree: false,
                enable_compressed_quad: false,
                rtree_min_segments: 64,
            },
        )
    };
    static ref DEFAULT_FINDER_RTREE_ONLY: DefaultFinder = DefaultFinder {
        finder: {
            let tzs = pbgen::Timezones::try_from(load_reduced()).unwrap_or_default();
            Finder::from_pb_with_index_options(
                tzs,
                PolygonBuildOptions {
                    enable_rtree: true,
                    enable_compressed_quad: false,
                    rtree_min_segments: 64,
                },
            )
        },
        fuzzy_finder: FuzzyFinder::default(),
    };
    static ref DEFAULT_FINDER_QUAD_ONLY: DefaultFinder = DefaultFinder {
        finder: {
            let tzs = pbgen::Timezones::try_from(load_reduced()).unwrap_or_default();
            Finder::from_pb_with_index_options(
                tzs,
                PolygonBuildOptions {
                    enable_rtree: false,
                    enable_compressed_quad: true,
                    rtree_min_segments: 64,
                },
            )
        },
        fuzzy_finder: FuzzyFinder::default(),
    };
    static ref DEFAULT_FINDER_NO_INDEX: DefaultFinder = DefaultFinder {
        finder: {
            let tzs = pbgen::Timezones::try_from(load_reduced()).unwrap_or_default();
            Finder::from_pb_with_index_options(
                tzs,
                PolygonBuildOptions {
                    enable_rtree: false,
                    enable_compressed_quad: false,
                    rtree_min_segments: 64,
                },
            )
        },
        fuzzy_finder: FuzzyFinder::default(),
    };
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

fn bench_finders(c: &mut Criterion) {
    let mut group = c.benchmark_group("Finders");

    // warmup
    let _ = DEFAULT_FINDER.get_tz_name(116.3883, 39.9289);
    let _ = FINDER.get_tz_name(116.3883, 39.9289);

    let i = &0;
    group.bench_with_input(BenchmarkId::new("Default", i), i, |b, _i| {
        b.iter(|| bench_default_finder_random_city());
    });
    group.bench_with_input(BenchmarkId::new("Finder", i), i, |b, _i| {
        b.iter(|| bench_finder_random_city());
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
