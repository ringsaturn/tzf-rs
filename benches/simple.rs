#![feature(test)]
#[cfg(test)]
mod benches_default {

    use std::fs::File;
    use tzf_rs::DefaultFinder;
    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_default_finder_random_city(b: &mut Bencher) {
        let guard = pprof::ProfilerGuardBuilder::default()
            .frequency(3000)
            .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build()
            .unwrap();
        let finder: DefaultFinder = DefaultFinder::default();

        b.iter(|| {
            let city = cities_json::get_random_cities();
            let _ = finder.get_tz_name(city.lng, city.lat);
        });
        if let Ok(report) = guard.report().build() {
            let file = File::create("flamegraph.svg").unwrap();
            report.flamegraph(file).unwrap();
        };
    }
}
