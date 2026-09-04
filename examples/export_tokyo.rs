/// Example: Export Asia/Tokyo timezone data to a GeoJSON file
///
/// Exports Asia/Tokyo boundary data to the tmp directory.
use std::fs;
use tzf_rs::DefaultFinder;

fn main() {
    // Create tmp directory if it doesn't exist
    fs::create_dir_all("tmp").expect("Failed to create tmp directory");

    println!("=== Exporting Asia/Tokyo ===");
    let finder = DefaultFinder::new();

    let Some(collection) = finder.get_tz_geojson("Asia/Tokyo") else {
        println!("✗ Asia/Tokyo not found");
        return;
    };

    println!("Found Asia/Tokyo: {} feature(s)", collection.features.len());
    if let Some(feature) = collection.features.first() {
        println!("Number of polygons: {}", feature.geometry.coordinates.len());
    }

    let json_string = collection.to_string_pretty();
    fs::write("tmp/tokyo.geojson", &json_string).expect("Failed to write GeoJSON file");

    println!("✓ Saved to tmp/tokyo.geojson");
    println!("  File size: {} bytes", json_string.len());
}
