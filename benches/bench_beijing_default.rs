#![feature(test)]
#[cfg(test)]
mod benches {

    use std::fs::File;
    use tzf_rs::DefaultFinder;
    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_get_tz_beijing(b: &mut Bencher) {
        let guard = pprof::ProfilerGuardBuilder::default()
            .frequency(1000)
            .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build()
            .unwrap();

        let finder: DefaultFinder = DefaultFinder::new_default();

        b.iter(|| {
            let _ = finder.get_tz_name(116.3883, 39.9289);
        });

        if let Ok(report) = guard.report().build() {
            let file = File::create("flamegraph.svg").unwrap();
            report.flamegraph(file).unwrap();
        };
    }
}
