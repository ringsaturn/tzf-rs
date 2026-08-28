/// Example: Convert timezone data to GeoJSON format
///
/// Demonstrates the GeoJSON conversion methods on both finder mechanisms.
use tzf_rs::{DefaultFinder, EmbeddedFinder};

fn main() {
    // Example 1: Convert DefaultFinder data to GeoJSON
    println!("=== Example 1: DefaultFinder to GeoJSON ===");
    let finder = DefaultFinder::new();
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

    // Example 2: EmbeddedFinder exports the same data, decoding on demand
    println!("\n\n=== Example 2: EmbeddedFinder to GeoJSON ===");
    let embedded = EmbeddedFinder::new();
    let embedded_geojson = embedded.to_geojson();

    println!("Type: {}", embedded_geojson.collection_type);
    println!("Number of features: {}", embedded_geojson.features.len());
}
