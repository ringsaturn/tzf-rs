/// Example: Export specific timezones to GeoJSON files
///
/// This example demonstrates how to use the `get_tz_geojson` method
/// to export specific timezones from Finder and FuzzyFinder.
use std::fs;
use tzf_rs::{DefaultFinder, Finder, FuzzyFinder};

fn main() {
    // Create tmp directory if it doesn't exist
    fs::create_dir_all("tmp").expect("Failed to create tmp directory");

    println!("=== Exporting Specific Timezones to GeoJSON ===\n");

    // List of timezones to export
    let timezones = vec![
        "Asia/Tokyo",
        "America/New_York",
        "Europe/London",
        "Asia/Shanghai",
        "Australia/Sydney",
    ];

    // Export from Finder
    println!("--- Using Finder (Precise Boundaries) ---");
    let finder = Finder::new();

    for tz_name in &timezones {
        if let Some(collection) = finder.get_tz_geojson(tz_name) {
            let json_string =
                serde_json::to_string_pretty(&collection).expect("Failed to serialize to JSON");

            let safe_name = tz_name.replace('/', "_");
            let filename = format!("tmp/{}_finder.geojson", safe_name);

            fs::write(&filename, &json_string).expect("Failed to write file");

            let polygon_count: usize = collection
                .features
                .iter()
                .map(|f| f.geometry.coordinates.len())
                .sum();

            println!(
                "✓ {} -> {} ({} bytes, {} feature(s), {} polygon(s))",
                tz_name,
                filename,
                json_string.len(),
                collection.features.len(),
                polygon_count
            );
        } else {
            println!("✗ {} not found", tz_name);
        }
    }

    // Export from FuzzyFinder
    println!("\n--- Using FuzzyFinder (Tile-Based Approximation) ---");
    let fuzzy_finder = FuzzyFinder::new();

    for tz_name in &timezones {
        if let Some(feature) = fuzzy_finder.get_tz_geojson(tz_name) {
            let json_string =
                serde_json::to_string_pretty(&feature).expect("Failed to serialize to JSON");

            let safe_name = tz_name.replace('/', "_");
            let filename = format!("tmp/{}_fuzzy.geojson", safe_name);

            fs::write(&filename, &json_string).expect("Failed to write file");

            println!(
                "✓ {} -> {} ({} bytes, {} tiles)",
                tz_name,
                filename,
                json_string.len(),
                feature.geometry.coordinates.len()
            );
        } else {
            println!("✗ {} not found", tz_name);
        }
    }

    // Export from DefaultFinder
    println!("\n--- Using DefaultFinder ---");
    let default_finder = DefaultFinder::new();

    // Just export one as example
    let example_tz = "Asia/Tokyo";
    if let Some(collection) = default_finder.get_tz_geojson(example_tz) {
        let json_string =
            serde_json::to_string_pretty(&collection).expect("Failed to serialize to JSON");

        let filename = "tmp/tokyo_default.geojson";
        fs::write(filename, &json_string).expect("Failed to write file");

        println!(
            "✓ {} -> {} ({} bytes, {} feature(s))",
            example_tz,
            filename,
            json_string.len(),
            collection.features.len()
        );
    }

    // Demonstrate error handling
    println!("\n--- Error Handling Example ---");
    let invalid_tz = "Invalid/Timezone";
    match finder.get_tz_geojson(invalid_tz) {
        Some(_) => println!("Found {}", invalid_tz),
        None => println!("✗ {} not found (as expected)", invalid_tz),
    }

    println!("\n=== Summary ===");
    println!("All GeoJSON files have been saved to the tmp directory.");
    println!("Files:");
    println!("  - Finder files: *_finder.geojson (precise boundaries)");
    println!("  - FuzzyFinder files: *_fuzzy.geojson (tile-based)");
    println!("  - DefaultFinder file: tokyo_default.geojson");
}
