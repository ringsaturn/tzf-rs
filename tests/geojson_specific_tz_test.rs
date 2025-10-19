#![cfg(feature = "export-geojson")]

use tzf_rs::{DefaultFinder, Finder, FuzzyFinder};

#[test]
fn test_finder_get_tz_geojson_found() {
    let finder = Finder::new();
    let result = finder.get_tz_geojson("Asia/Tokyo");

    assert!(result.is_some());
    let collection = result.unwrap();
    assert_eq!(collection.collection_type, "FeatureCollection");
    assert!(!collection.features.is_empty());

    let first_feature = &collection.features[0];
    assert_eq!(first_feature.feature_type, "Feature");
    assert_eq!(first_feature.properties.tzid, "Asia/Tokyo");
    assert_eq!(first_feature.geometry.geometry_type, "MultiPolygon");
    assert!(!first_feature.geometry.coordinates.is_empty());

    // Verify we can serialize to JSON
    let json_string = collection.to_string_pretty();
    assert!(!json_string.is_empty());
}

#[test]
fn test_finder_get_tz_geojson_not_found() {
    let finder = Finder::new();
    let result = finder.get_tz_geojson("Invalid/Timezone");

    assert!(result.is_none());
}

#[test]
fn test_fuzzy_finder_get_tz_geojson_found() {
    let finder = FuzzyFinder::new();
    let result = finder.get_tz_geojson("Asia/Tokyo");

    assert!(result.is_some());
    let feature = result.unwrap();
    assert_eq!(feature.feature_type, "Feature");
    assert_eq!(feature.properties.tzid, "Asia/Tokyo");
    assert_eq!(feature.geometry.geometry_type, "MultiPolygon");
    assert!(!feature.geometry.coordinates.is_empty());

    // Verify we can serialize to JSON
    let json_string = feature.to_string_pretty();
    assert!(!json_string.is_empty());
}

#[test]
fn test_fuzzy_finder_get_tz_geojson_not_found() {
    let finder = FuzzyFinder::new();
    let result = finder.get_tz_geojson("Invalid/Timezone");

    assert!(result.is_none());
}

#[test]
fn test_default_finder_get_tz_geojson_found() {
    let finder = DefaultFinder::new();
    let result = finder.get_tz_geojson("Asia/Tokyo");

    assert!(result.is_some());
    let collection = result.unwrap();
    assert_eq!(collection.collection_type, "FeatureCollection");
    assert!(!collection.features.is_empty());

    let first_feature = &collection.features[0];
    assert_eq!(first_feature.feature_type, "Feature");
    assert_eq!(first_feature.properties.tzid, "Asia/Tokyo");
    assert_eq!(first_feature.geometry.geometry_type, "MultiPolygon");
    assert!(!first_feature.geometry.coordinates.is_empty());
}

#[test]
fn test_default_finder_get_tz_geojson_not_found() {
    let finder = DefaultFinder::new();
    let result = finder.get_tz_geojson("Invalid/Timezone");

    assert!(result.is_none());
}

#[test]
fn test_multiple_timezones() {
    let finder = Finder::new();

    let timezones = vec![
        "Asia/Tokyo",
        "America/New_York",
        "Europe/London",
        "Asia/Shanghai",
    ];

    for tz_name in timezones {
        let result = finder.get_tz_geojson(tz_name);
        assert!(result.is_some(), "Should find timezone: {}", tz_name);

        let collection = result.unwrap();
        assert!(!collection.features.is_empty());
        assert_eq!(collection.features[0].properties.tzid, tz_name);
    }
}

#[test]
fn test_geojson_specific_tz_structure() {
    let finder = Finder::new();
    let result = finder.get_tz_geojson("Asia/Shanghai");

    assert!(result.is_some());
    let collection = result.unwrap();
    assert!(!collection.features.is_empty());

    let feature = &collection.features[0];

    // Verify polygon structure
    assert!(!feature.geometry.coordinates.is_empty());

    for polygon in &feature.geometry.coordinates {
        assert!(
            !polygon.is_empty(),
            "Polygon should have at least an exterior ring"
        );

        for (ring_idx, ring) in polygon.iter().enumerate() {
            // First ring (exterior) should have at least 3 points
            // Other rings (holes) might be empty in some cases
            if ring_idx == 0 {
                assert!(
                    ring.len() >= 3,
                    "Exterior ring should have at least 3 points"
                );
            }

            for point in ring {
                assert_eq!(point.len(), 2, "Point should have [lng, lat] format");
                assert!(
                    point[0] >= -180.0 && point[0] <= 180.0,
                    "Longitude should be valid"
                );
                assert!(
                    point[1] >= -90.0 && point[1] <= 90.0,
                    "Latitude should be valid"
                );
            }
        }
    }
}

#[test]
fn test_compare_finder_vs_fuzzy_same_timezone() {
    let finder = Finder::new();
    let fuzzy_finder = FuzzyFinder::new();

    let tz_name = "Asia/Tokyo";

    let finder_result = finder.get_tz_geojson(tz_name);
    let fuzzy_result = fuzzy_finder.get_tz_geojson(tz_name);

    assert!(finder_result.is_some());
    assert!(fuzzy_result.is_some());

    let finder_collection = finder_result.unwrap();
    let fuzzy_feature = fuzzy_result.unwrap();

    // Finder returns a collection, fuzzy returns a single feature
    assert!(!finder_collection.features.is_empty());
    assert_eq!(
        finder_collection.features[0].properties.tzid,
        fuzzy_feature.properties.tzid
    );

    // Fuzzy finder typically has more polygons (tiles) than precise finder
    let finder_polygon_count: usize = finder_collection
        .features
        .iter()
        .map(|f| f.geometry.coordinates.len())
        .sum();
    println!(
        "Finder polygons: {}, FuzzyFinder polygons: {}",
        finder_polygon_count,
        fuzzy_feature.geometry.coordinates.len()
    );
}
