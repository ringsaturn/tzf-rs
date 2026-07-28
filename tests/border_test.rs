//! Regression tests for <https://github.com/ringsaturn/tzf-rs/issues/207>:
//! a query landing exactly on a shared polygon border used to match neither
//! neighbour and return an empty result.

#[cfg(test)]
mod tests {
    use tzf_rs::{DefaultFinder, Finder};

    #[test]
    fn nautical_border_is_not_a_gap() {
        let finder = DefaultFinder::new();

        // The nautical zones are 15°-wide strips, so their borders sit on
        // whole meridians. The two coordinates from the issue report:
        for (lng, lat) in [(7.5, 54.5), (-22.5, 54.5)] {
            assert!(
                !finder.get_tz_name(lng, lat).is_empty(),
                "get_tz_name({lng}, {lat}) is empty"
            );
            // Both sides of the border claim the point.
            assert_eq!(
                finder.get_tz_names(lng, lat).len(),
                2,
                "get_tz_names({lng}, {lat}) = {:?}",
                finder.get_tz_names(lng, lat)
            );
        }

        assert_eq!(finder.get_tz_names(7.5, 54.5), ["Etc/GMT-1", "Etc/GMT"]);
        assert_eq!(finder.get_tz_names(-22.5, 54.5), ["Etc/GMT+1", "Etc/GMT+2"]);
    }

    #[test]
    fn border_neighbours_still_resolve_to_one_side_each() {
        let finder = DefaultFinder::new();

        assert_eq!(finder.get_tz_name(7.4999, 54.5), "Etc/GMT");
        assert_eq!(finder.get_tz_name(7.5001, 54.5), "Etc/GMT-1");
        assert_eq!(finder.get_tz_name(-22.4999, 54.5), "Etc/GMT+1");
        assert_eq!(finder.get_tz_name(-22.5001, 54.5), "Etc/GMT+2");
    }

    /// A 1°×1° sweep of the whole globe. Every point used to miss on the 24
    /// nautical meridians (2753 empty results); nothing may miss now.
    #[test]
    fn global_grid_has_no_holes() {
        let finder = Finder::new();

        let mut empty: Vec<(f64, f64)> = vec![];
        let mut lng = -179.5;
        while lng <= 179.5 {
            let mut lat = -89.5;
            while lat <= 89.5 {
                if finder.get_tz_name(lng, lat).is_empty() {
                    empty.push((lng, lat));
                }
                lat += 1.0;
            }
            lng += 1.0;
        }

        assert!(
            empty.is_empty(),
            "{} empty results, first few: {:?}",
            empty.len(),
            &empty[..empty.len().min(10)]
        );
    }
}
