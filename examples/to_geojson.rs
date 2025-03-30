use std::fs::File;
use std::io::Write;
use tzf_rs::{DefaultFinder, Finder, FuzzyFinder};

fn main() -> std::io::Result<()> {
    // Example 1: Using DefaultFinder (recommended)
    println!("Converting using DefaultFinder...");
    let finder = DefaultFinder::new();
    let geojson = finder.to_geojson();
    let json_str = serde_json::to_string_pretty(&geojson)?;
    let mut file = File::create("default_finder_tz.geojson")?;
    file.write_all(json_str.as_bytes())?;
    println!("Saved to default_finder_tz.geojson");

    // Example 2: Using Finder
    println!("\nConverting using Finder...");
    let finder = Finder::new();
    let geojson = finder.to_geojson();
    let json_str = serde_json::to_string_pretty(&geojson)?;
    let mut file = File::create("finder_tz.geojson")?;
    file.write_all(json_str.as_bytes())?;
    println!("Saved to finder_tz.geojson");

    // Example 3: Using FuzzyFinder
    println!("\nConverting using FuzzyFinder...");
    let finder = FuzzyFinder::new();
    let geojson = finder.to_geojson();
    let json_str = serde_json::to_string_pretty(&geojson)?;
    let mut file = File::create("fuzzy_finder_tz.geojson")?;
    file.write_all(json_str.as_bytes())?;
    println!("Saved to fuzzy_finder_tz.geojson");

    // Example 4: Convert specific timezone using different finders
    let timezone = "Asia/Shanghai";
    println!("\nConverting specific timezone: {}", timezone);

    // Using DefaultFinder
    let finder = DefaultFinder::new();
    if let Some(geojson) = finder.timezone_to_geojson(timezone) {
        let json_str = serde_json::to_string_pretty(&geojson)?;
        let mut file = File::create("shanghai_default.geojson")?;
        file.write_all(json_str.as_bytes())?;
        println!("Saved to shanghai_default.geojson");
    }

    // Using Finder
    let finder = Finder::new();
    if let Some(geojson) = finder.timezone_to_geojson(timezone) {
        let json_str = serde_json::to_string_pretty(&geojson)?;
        let mut file = File::create("shanghai_finder.geojson")?;
        file.write_all(json_str.as_bytes())?;
        println!("Saved to shanghai_finder.geojson");
    }

    // Using FuzzyFinder
    let finder = FuzzyFinder::new();
    if let Some(geojson) = finder.timezone_to_geojson(timezone) {
        let json_str = serde_json::to_string_pretty(&geojson)?;
        let mut file = File::create("shanghai_fuzzy.geojson")?;
        file.write_all(json_str.as_bytes())?;
        println!("Saved to shanghai_fuzzy.geojson");
    }

    // Example 5: Verify the data by checking a known timezone
    let finder = DefaultFinder::new();
    let tz_name = finder.get_tz_name(116.3883, 39.9289);
    println!("\nVerification: Beijing timezone = {}", tz_name);

    Ok(())
} 