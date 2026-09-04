#[cfg(test)]
#[cfg(feature = "bundled")]
mod tests {
    use tzf_rs::{DefaultFinder, EmbeddedFinder, deg2num};

    fn assert_known_locations(get: impl Fn(f64, f64) -> String) {
        assert_eq!(get(116.3883, 39.9289), "Asia/Shanghai");
        assert_eq!(get(121.3547, 31.1139), "Asia/Shanghai");
        assert_eq!(get(111.8674, 34.4200), "Asia/Shanghai");
        assert_eq!(get(-97.8674, 34.4200), "America/Chicago");
        assert_eq!(get(139.4382, 36.4432), "Asia/Tokyo");
        assert_eq!(get(24.5212, 50.2506), "Europe/Kyiv");
        assert_eq!(get(-0.9671, 52.0152), "Europe/London");
        assert_eq!(get(-4.5706, 46.2747), "Etc/GMT");
        assert_eq!(get(-73.7729, 38.3530), "Etc/GMT+5");
        assert_eq!(get(114.1594, 22.3173), "Asia/Hong_Kong");
        assert_eq!(get(9.8198, 27.5775), "Africa/Tripoli");

        // Original GCJ-02 coordinates: [114.0668, 22.5153], which is in
        // Shenzhen, China, and very close to the border with Hong Kong.
        // Reverted to WGS-84 coordinates to get the correct timezone.
        //
        // AMAP link: https://surl.amap.com/uJcx40w1e6bd
        assert_eq!(get(114.0617, 22.5180), "Asia/Shanghai");

        assert_eq!(
            get(12.452_899_553_691_935, 41.903_699_636_969_634),
            "Europe/Vatican"
        );

        // Locations that used to fall into simplification gaps.
        assert!(!get(8.61280918, 47.66097966).is_empty());
        assert!(!get(8.61231565, 47.66148548).is_empty());
    }

    #[test]
    fn default_finder_smoke_test() {
        let finder = DefaultFinder::new();
        assert_known_locations(|lng, lat| finder.get_tz_name(lng, lat).to_string());
        assert_eq!(finder.data_version(), "2026c");
        assert!(!finder.timezonenames().is_empty());
    }

    #[test]
    fn embedded_finder_smoke_test() {
        let finder = EmbeddedFinder::new();
        assert_known_locations(|lng, lat| finder.get_tz_name(lng, lat).to_string());
        assert_eq!(finder.data_version(), "2026c");
        assert!(!finder.timezonenames().is_empty());
    }

    #[test]
    fn embedded_finder_over_owned_bytes() {
        let data = tzf_dist::load_lite_tzb().to_vec();
        let finder = EmbeddedFinder::from_tzb(data).expect("valid embedded data");
        assert_eq!(finder.get_tz_name(116.3883, 39.9289), "Asia/Shanghai");
    }

    #[test]
    fn out_of_domain_queries_return_empty() {
        let finder = DefaultFinder::new();
        let embedded = EmbeddedFinder::new();
        for (lng, lat) in [
            (f64::NAN, 39.9),
            (116.4, f64::NAN),
            (f64::INFINITY, 0.0),
            (0.0, f64::NEG_INFINITY),
            (-180.5, 0.0),
            (180.5, 0.0),
            (0.0, 90.5),
            (0.0, -90.5),
        ] {
            assert_eq!(finder.get_tz_name(lng, lat), "");
            assert!(finder.get_tz_names(lng, lat).is_empty());
            assert_eq!(embedded.get_tz_name(lng, lat), "");
            assert!(embedded.get_tz_names(lng, lat).is_empty());
        }
    }

    #[test]
    fn multi_results_are_name_sorted() {
        let finder = DefaultFinder::new();
        let embedded = EmbeddedFinder::new();
        for (lng, lat) in [(7.5, 54.5), (-22.5, 54.5), (87.4160, 44.0400)] {
            let names = finder.get_tz_names(lng, lat);
            let mut sorted = names.clone();
            sorted.sort_unstable();
            assert_eq!(names, sorted, "unsorted names at ({lng}, {lat})");
            assert_eq!(
                embedded.get_tz_names(lng, lat),
                names,
                "mechanism mismatch at ({lng}, {lat})"
            );
        }
    }

    #[test]
    fn test_deg2num() {
        assert_eq!(deg2num(116.3883, 39.9289, 7), (105, 48));
    }
}
