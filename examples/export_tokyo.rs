/// Example: Export Asia/Tokyo timezone data to GeoJSON files
///
/// This example exports Asia/Tokyo timezone data from both Finder and FuzzyFinder
/// to separate GeoJSON files in the tmp directory.
use std::fs;
use tzf_rs::{Finder, FuzzyFinder};

fn main() {
    // Create tmp directory if  it doesn't exist
    fs::create_dir_all("tmp").expect("Failed to create tmp directory");

    // Export from Finder
    println!("=== Exporting Asia/Tokyo from Finder ===");
    let finder = Finder::new();
    let finder_geojson = finder.to_geojson();

    // Find Asia/Tokyo in the features
    let tokyo_finder = finder_geojson
        .features
        .iter()
        .find(|f| f.properties.tzid == "Asia/Tokyo");

    if let Some(feature) = tokyo_finder {
        println!("Found Asia/Tokyo in Finder");
        println!("Number of polygons: {}", feature.geometry.coordinates.len());

        // Create a FeatureCollection with just this timezone
        let single_feature_collection = tzf_rs::BoundaryFile {
            collection_type: "FeatureCollection".to_string(),
            features: vec![feature.clone()],
        };

        let json_string = single_feature_collection.to_string_pretty();

        fs::write("tmp/tokyo_finder.geojson", &json_string)
            .expect("Failed to write finder GeoJSON file");

        println!("✓ Saved to tmp/tokyo_finder.geojson");
        println!("  File size: {} bytes", json_string.len());
    } else {
        println!("✗ Asia/Tokyo not found in Finder");
    }

    // Export from FuzzyFinder
    println!("\n=== Exporting Asia/Tokyo from FuzzyFinder ===");
    let fuzzy_finder = FuzzyFinder::new();
    let fuzzy_geojson = fuzzy_finder.to_geojson();

    // Find Asia/Tokyo in the features
    let tokyo_fuzzy = fuzzy_geojson
        .features
        .iter()
        .find(|f| f.properties.tzid == "Asia/Tokyo");

    if let Some(feature) = tokyo_fuzzy {
        println!("Found Asia/Tokyo in FuzzyFinder");
        println!(
            "Number of tile polygons: {}",
            feature.geometry.coordinates.len()
        );

        // Create a FeatureCollection with just this timezone
        let single_feature_collection = tzf_rs::BoundaryFile {
            collection_type: "FeatureCollection".to_string(),
            features: vec![feature.clone()],
        };

        let json_string = single_feature_collection.to_string_pretty();

        fs::write("tmp/tokyo_fuzzy_finder.geojson", &json_string)
            .expect("Failed to write fuzzy finder GeoJSON file");

        println!("✓ Saved to tmp/tokyo_fuzzy_finder.geojson");
        println!("  File size: {} bytes", json_string.len());
    } else {
        println!("✗ Asia/Tokyo not found in FuzzyFinder");
    }

    println!("\n=== Summary ===");
    println!("Both GeoJSON files have been saved to the tmp directory:");
    println!("  - tmp/tokyo_finder.geojson (precise boundaries)");
    println!("  - tmp/tokyo_fuzzy_finder.geojson (tile-based approximation)");
}
