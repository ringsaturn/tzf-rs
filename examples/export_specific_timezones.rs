/// Example: Export specific timezones to individual GeoJSON files
///
/// Exports a handful of timezones to separate files under tmp/.
use std::fs;
use tzf_rs::DefaultFinder;

fn main() {
    fs::create_dir_all("tmp").expect("Failed to create tmp directory");

    let finder = DefaultFinder::new();
    let timezones = [
        "Asia/Shanghai",
        "Asia/Tokyo",
        "America/New_York",
        "Europe/London",
    ];

    for tz_name in timezones {
        let Some(collection) = finder.get_tz_geojson(tz_name) else {
            println!("✗ {tz_name} not found");
            continue;
        };
        let filename = format!("tmp/{}.geojson", tz_name.replace('/', "_"));
        let json_string = collection.to_string_pretty();
        fs::write(&filename, &json_string).expect("Failed to write GeoJSON file");
        println!(
            "✓ {tz_name}: {} feature(s), {} bytes → {filename}",
            collection.features.len(),
            json_string.len()
        );
    }
}
