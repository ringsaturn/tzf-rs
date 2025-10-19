/// Example: Convert timezone data to GeoJSON format
///
/// This example demonstrates how to use the GeoJSON conversion functions
/// for Finder, FuzzyFinder, and DefaultFinder.
use tzf_rs::{DefaultFinder, Finder, FuzzyFinder};

fn main() {
    // Example 1: Convert Finder data to GeoJSON
    println!("=== Example 1: Finder to GeoJSON ===");
    let finder = Finder::new();
    let geojson = finder.to_geojson();

    println!("Type: {}", geojson.collection_type);
    println!("Number of features: {}", geojson.features.len());

    if let Some(first_feature) = geojson.features.first() {
        println!("First timezone: {}", first_feature.properties.tzid);
        println!(
            "Number of polygons: {}",
            first_feature.geometry.coordinates.len()
        );
    }

    // Serialize to JSON string
    let json_string = geojson.to_string_pretty();
    println!("\nGeoJSON preview (first 500 chars):");
    println!("{}", &json_string[..json_string.len().min(500)]);

    // Example 2: Convert FuzzyFinder data to GeoJSON
    println!("\n\n=== Example 2: FuzzyFinder to GeoJSON ===");
    let fuzzy_finder = FuzzyFinder::new();
    let fuzzy_geojson = fuzzy_finder.to_geojson();

    println!("Type: {}", fuzzy_geojson.collection_type);
    println!("Number of features: {}", fuzzy_geojson.features.len());

    // Example 3: Convert DefaultFinder data to GeoJSON
    println!("\n\n=== Example 3: DefaultFinder to GeoJSON ===");
    let default_finder = DefaultFinder::new();
    let default_geojson = default_finder.to_geojson();

    println!("Type: {}", default_geojson.collection_type);
    println!("Number of features: {}", default_geojson.features.len());

    // Example 4: Save to a file (optional, commented out)
    // std::fs::write("timezones.geojson", json_string).unwrap();
    // println!("\nGeoJSON saved to timezones.geojson");

    // Example 5: Find a specific timezone and export just that one
    println!("\n\n=== Example 4: Export specific timezone ===");
    let shanghai_feature: Option<&tzf_rs::FeatureItem> = geojson
        .features
        .iter()
        .find(|f| f.properties.tzid == "Asia/Shanghai");

    if let Some(feature) = shanghai_feature {
        println!("Found timezone: {}", feature.properties.tzid);
        let single_feature_json = feature.to_string_pretty();
        println!("Feature JSON (first 300 chars):");
        println!(
            "{}",
            &single_feature_json[..single_feature_json.len().min(300)]
        );
    }
}
