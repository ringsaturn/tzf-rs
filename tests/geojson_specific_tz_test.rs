#![cfg(all(feature = "export-geojson", feature = "bundled"))]

use tzf_rs::{DefaultFinder, EmbeddedFinder};

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

    // Verify we can serialize to JSON
    let json_string = collection.to_string_pretty();
    assert!(!json_string.is_empty());
}

#[test]
fn test_default_finder_get_tz_geojson_not_found() {
    let finder = DefaultFinder::new();
    assert!(finder.get_tz_geojson("Invalid/Timezone").is_none());
}

#[test]
fn test_embedded_finder_get_tz_geojson_matches_default() {
    let expanded = DefaultFinder::new();
    let inplace = EmbeddedFinder::new();

    for name in ["Asia/Tokyo", "Europe/Vatican", "America/Chicago"] {
        let a = expanded.get_tz_geojson(name).expect("timezone exists");
        let b = inplace.get_tz_geojson(name).expect("timezone exists");
        // The in-place export decodes rings on demand; the JSON must be
        // byte-identical to the expanded finder's.
        assert_eq!(a.to_string(), b.to_string(), "export mismatch for {name}");
    }
    assert!(inplace.get_tz_geojson("Invalid/Timezone").is_none());
}
