#[cfg(test)]
mod tests {
    use tzf_rs::Finder;

    #[test]
    fn smoke_test() {
        let finder = Finder::new();

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
    }
}
