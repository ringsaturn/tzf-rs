#![cfg(feature = "export-geojson")]

use tzf_rs::{DefaultFinder, Finder, FuzzyFinder};

#[test]
fn test_finder_to_geojson() {
    let finder = Finder::new();
    let geojson = finder.to_geojson();

    assert_eq!(geojson.collection_type, "FeatureCollection");
    assert!(!geojson.features.is_empty());

    // Verify we can serialize to JSON
    let json_string = serde_json::to_string(&geojson).unwrap();
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
fn test_fuzzy_finder_to_geojson() {
    let finder = FuzzyFinder::new();
    let geojson = finder.to_geojson();

    assert_eq!(geojson.collection_type, "FeatureCollection");
    assert!(!geojson.features.is_empty());

    // Verify we can serialize to JSON
    let json_string = serde_json::to_string(&geojson).unwrap();
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
fn test_default_finder_to_geojson() {
    let finder = DefaultFinder::new();
    let geojson = finder.to_geojson();

    assert_eq!(geojson.collection_type, "FeatureCollection");
    assert!(!geojson.features.is_empty());

    // Verify we can serialize to JSON
    let json_string = serde_json::to_string(&geojson).unwrap();
    assert!(!json_string.is_empty());
}

#[test]
fn test_geojson_contains_specific_timezone() {
    let finder = Finder::new();
    let geojson = finder.to_geojson();

    // Check if Asia/Shanghai is present
    let has_shanghai = geojson
        .features
        .iter()
        .any(|f| f.properties.tzid == "Asia/Shanghai");
    assert!(has_shanghai, "Should contain Asia/Shanghai timezone");
}

#[test]
fn test_geojson_polygon_structure() {
    let finder = Finder::new();
    let geojson = finder.to_geojson();

    // Get the first feature and verify the coordinate structure
    if let Some(first_feature) = geojson.features.first() {
        assert!(!first_feature.geometry.coordinates.is_empty());

        // Each polygon in MultiPolygon should have at least an exterior ring
        for polygon in &first_feature.geometry.coordinates {
            assert!(
                !polygon.is_empty(),
                "Polygon should have at least an exterior ring"
            );

            // Each ring should have at least 3 points (triangle)
            for ring in polygon {
                assert!(ring.len() >= 3, "Ring should have at least 3 points");

                // Each point should be [lng, lat]
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
}
