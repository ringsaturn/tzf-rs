#![cfg(all(feature = "export-geojson", feature = "bundled"))]

use tzf_rs::{DefaultFinder, EmbeddedFinder};

#[test]
fn test_default_finder_to_geojson() {
    let finder = DefaultFinder::new();
    let geojson = finder.to_geojson();

    assert_eq!(geojson.collection_type, "FeatureCollection");
    assert!(!geojson.features.is_empty());

    // Verify we can serialize to JSON
    let json_string = geojson.to_string();
    assert!(!json_string.is_empty());

    // Verify structure
    for feature in &geojson.features {
        assert_eq!(feature.feature_type, "Feature");
        assert_eq!(feature.geometry.geometry_type, "MultiPolygon");
        assert!(!feature.properties.tzid.is_empty());
        assert!(!feature.geometry.coordinates.is_empty());
    }
}

#[test]
fn test_embedded_finder_to_geojson_matches_default() {
    let expanded = DefaultFinder::new().to_geojson();
    let inplace = EmbeddedFinder::new().to_geojson();

    // Both mechanisms export the identical expansion of the same file.
    assert_eq!(expanded.to_string(), inplace.to_string());
}

#[test]
fn test_geojson_roundtrip() {
    let finder = DefaultFinder::new();
    let geojson = finder
        .get_tz_geojson("Asia/Tokyo")
        .expect("Asia/Tokyo exists");
    let json_string = geojson.to_string();

    let parsed: tzf_rs::BoundaryFile = serde_json::from_str(&json_string).expect("valid JSON");
    assert_eq!(parsed.collection_type, "FeatureCollection");
    assert_eq!(parsed.features.len(), geojson.features.len());
}
