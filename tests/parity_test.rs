//! Mechanism parity: every lookup mechanism over the same dataset must
//! return identical results — the Rust-side analogue of the Go pipeline's
//! `embedcompare` gate. `DefaultFinder` expands the `.tzb` geometry into
//! geometry-rs polygons while `EmbeddedFinder` walks the compressed chunk
//! streams in place with its own raycast port, so this sweep pins the two
//! implementations (including boundary behavior) to each other.

#[cfg(test)]
#[cfg(feature = "bundled")]
mod tests {
    use cities_json::CITIES;
    use tzf_rs::{DefaultFinder, EmbeddedFinder};

    fn assert_same(expanded: &DefaultFinder, inplace: &EmbeddedFinder, lng: f64, lat: f64) {
        assert_eq!(
            expanded.get_tz_name(lng, lat),
            inplace.get_tz_name(lng, lat),
            "get_tz_name mismatch at ({lng}, {lat})"
        );
        assert_eq!(
            expanded.get_tz_names(lng, lat),
            inplace.get_tz_names(lng, lat),
            "get_tz_names mismatch at ({lng}, {lat})"
        );
    }

    #[test]
    fn metadata_matches() {
        let expanded = DefaultFinder::new();
        let inplace = EmbeddedFinder::new();
        assert_eq!(expanded.data_version(), inplace.data_version());
        assert_eq!(expanded.timezonenames(), inplace.timezonenames());
    }

    #[test]
    fn world_cities_match() {
        let expanded = DefaultFinder::new();
        let inplace = EmbeddedFinder::new();
        for city in CITIES.iter() {
            assert_same(&expanded, &inplace, city.lng, city.lat);
        }
    }

    /// Boundary-biased samples: city coordinates jittered toward polygon
    /// edges, where the two raycast implementations disagree first if they
    /// disagree at all.
    #[test]
    fn boundary_biased_samples_match() {
        let expanded = DefaultFinder::new();
        let inplace = EmbeddedFinder::new();
        for city in CITIES.iter().step_by(7) {
            for (dx, dy) in [
                (0.01, 0.0),
                (-0.01, 0.0),
                (0.0, 0.01),
                (0.0, -0.01),
                (0.02, 0.02),
            ] {
                let lng = (city.lng + dx).clamp(-180.0, 180.0);
                let lat = (city.lat + dy).clamp(-90.0, 90.0);
                assert_same(&expanded, &inplace, lng, lat);
            }
        }
    }

    /// Exact whole-degree coordinates land on nautical-zone borders and grid
    /// cell corners — the hardest inputs for boundary parity.
    #[test]
    fn coarse_grid_matches_including_borders() {
        let expanded = DefaultFinder::new();
        let inplace = EmbeddedFinder::new();
        let mut lng = -180.0;
        while lng <= 180.0 {
            let mut lat = -90.0;
            while lat <= 90.0 {
                assert_same(&expanded, &inplace, lng, lat);
                assert_same(&expanded, &inplace, lng + 0.5, lat + 0.5);
                lat += 3.0;
            }
            lng += 3.0;
        }
    }
}
