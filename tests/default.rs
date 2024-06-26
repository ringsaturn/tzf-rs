#[cfg(test)]
mod tests {
    use tzf_rs::DefaultFinder;

    #[test]
    fn smoke_test() {
        let finder = DefaultFinder::new();

        assert_eq!(finder.get_tz_name(116.3883, 39.9289), "Asia/Shanghai");
        assert_eq!(finder.get_tz_name(121.3547, 31.1139), "Asia/Shanghai");
        assert_eq!(finder.get_tz_name(111.8674, 34.4200), "Asia/Shanghai");
        assert_eq!(finder.get_tz_name(-97.8674, 34.4200), "America/Chicago");
        assert_eq!(finder.get_tz_name(139.4382, 36.4432), "Asia/Tokyo");
        assert_eq!(finder.get_tz_name(24.5212, 50.2506), "Europe/Kyiv");
        assert_eq!(finder.get_tz_name(-0.9671, 52.0152), "Europe/London");
        assert_eq!(finder.get_tz_name(-4.5706, 46.2747), "Etc/GMT");
        assert_eq!(finder.get_tz_name(-4.5706, 46.2747), "Etc/GMT");
        assert_eq!(finder.get_tz_name(-73.7729, 38.3530), "Etc/GMT+5");
        assert_eq!(finder.get_tz_name(114.1594, 22.3173), "Asia/Hong_Kong");

        // Original GCJ-02 coordinates: [114.0668, 22.5153], which is in Shenzhen, China,
        // and very close to the border with Hong Kong.
        // Revert it to WGS-84 coordinates to get the correct timezone.
        //
        // AMAP link: https://surl.amap.com/uJcx40w1e6bd
        assert_eq!(finder.get_tz_name(114.0617, 22.5180), "Asia/Shanghai");

        assert_eq!(
            finder.get_tz_name(12.452_899_553_691_935, 41.903_699_636_969_634),
            "Europe/Vatican"
        );
    }
}
