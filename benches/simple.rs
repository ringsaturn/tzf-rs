#![feature(test)]
#[cfg(test)]
mod benches_default {

    use tzf_rs::DefaultFinder;
    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_default_finder_random_city(b: &mut Bencher) {
        let finder: DefaultFinder = DefaultFinder::default();

        b.iter(|| {
            let city = cities_json::get_random_cities();
            let _ = finder.get_tz_name(city.lng, city.lat);
        });
    }
}
