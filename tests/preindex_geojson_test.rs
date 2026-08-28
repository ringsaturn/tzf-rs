#![cfg(all(feature = "export-geojson", feature = "bundled"))]

use tzf_rs::{DefaultFinder, EmbeddedFinder};

#[test]
fn test_default_finder_get_tz_preindex_geojson_shape() {
    let finder = DefaultFinder::new();
    let collection = finder
        .get_tz_preindex_geojson("Asia/Shanghai")
        .expect("bundled data carries a FUZZY section");

    assert_eq!(collection.collection_type, "FeatureCollection");
    assert_eq!(collection.features.len(), 1);

    let feature = &collection.features[0];
    assert_eq!(feature.feature_type, "Feature");
    assert_eq!(feature.properties.tzid, "Asia/Shanghai");
    assert_eq!(feature.geometry.geometry_type, "MultiPolygon");
    assert!(!feature.geometry.coordinates.is_empty());

    // Every tile is a single closed 5-point rectangle ring.
    for polygon in &feature.geometry.coordinates {
        assert_eq!(polygon.len(), 1, "tile polygons carry no holes");
        let ring = &polygon[0];
        assert_eq!(ring.len(), 5);
        assert_eq!(ring[0], ring[4], "ring must be closed");
        assert_eq!(ring[0][1], ring[1][1], "bottom edge is horizontal");
        assert_eq!(ring[1][0], ring[2][0], "right edge is vertical");
    }

    // A point the preindex fast path answers must sit inside one tile bbox.
    let (lng, lat) = (116.3883, 39.9289);
    assert!(
        feature.geometry.coordinates.iter().any(|polygon| {
            let ring = &polygon[0];
            let (lng_min, lat_min) = (ring[0][0], ring[0][1]);
            let (lng_max, lat_max) = (ring[2][0], ring[2][1]);
            (lng_min..=lng_max).contains(&lng) && (lat_min..=lat_max).contains(&lat)
        }),
        "no preindex tile covers the Beijing sample point"
    );
}

#[test]
fn test_get_tz_preindex_geojson_unknown_name() {
    assert!(
        DefaultFinder::new()
            .get_tz_preindex_geojson("Invalid/Timezone")
            .is_none()
    );
    assert!(
        EmbeddedFinder::new()
            .get_tz_preindex_geojson("Invalid/Timezone")
            .is_none()
    );
}

#[test]
fn test_embedded_finder_preindex_geojson_matches_default() {
    let expanded = DefaultFinder::new();
    let inplace = EmbeddedFinder::new();

    for name in ["Asia/Shanghai", "Europe/Berlin", "America/Chicago"] {
        let a = expanded.get_tz_preindex_geojson(name).expect("tz has tiles");
        let b = inplace.get_tz_preindex_geojson(name).expect("tz has tiles");
        // Both finders order tiles by ascending packed key, so the JSON must
        // be byte-identical.
        assert_eq!(a.to_string(), b.to_string(), "export mismatch for {name}");
    }
}

#[test]
fn test_to_preindex_geojson_matches_between_finders() {
    let a = DefaultFinder::new()
        .to_preindex_geojson()
        .expect("bundled data carries a FUZZY section");
    let b = EmbeddedFinder::new()
        .to_preindex_geojson()
        .expect("bundled data carries a FUZZY section");
    assert!(!a.features.is_empty());
    assert_eq!(a.to_string(), b.to_string());
}
