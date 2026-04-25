#[cfg(test)]
#[cfg(feature = "full")]
mod tests {
    use tzf_rs::DefaultFinder;

    #[test]
    fn smoke_test() {
        let finder = DefaultFinder::new_full();
        assert_eq!(finder.get_tz_name(116.3883, 39.9289), "Asia/Shanghai");
        assert_eq!(finder.get_tz_name(139.4382, 36.4432), "Asia/Tokyo");
        assert_eq!(finder.get_tz_name(-97.8674, 34.4200), "America/Chicago");
        assert!(!finder.data_version().is_empty());
        assert!(!finder.timezonenames().is_empty());
    }
}
