#![feature(test)]
#[cfg(test)]
mod benches_finder {
    use tzf_rs::Finder;
    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_finder_get_tz_beijing(b: &mut Bencher) {
        let finder: Finder = Finder::new_default();

        b.iter(|| {
            let _ = finder.get_tz_name(116.3883, 39.9289);
        });
    }
}
