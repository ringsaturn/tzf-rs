/// Scans all cities in the cities-json dataset, collects those where FuzzyFinder
/// returns no result (preindex cache miss), and writes them to benches/edges.json.
///
/// Run with:
///   cargo run --example gen_edges
use cities_json::CITIES;
use serde::Serialize;
use std::fs;
use tzf_rs::FuzzyFinder;

#[derive(Serialize)]
struct EdgeCity {
    lng: f64,
    lat: f64,
    name: String,
    country: String,
}

fn main() {
    let fuzzy = FuzzyFinder::default();

    let edges: Vec<EdgeCity> = CITIES
        .iter()
        .filter(|city| fuzzy.get_tz_name(city.lng, city.lat).is_empty())
        .map(|city| EdgeCity {
            lng: city.lng,
            lat: city.lat,
            name: city.name.clone(),
            country: city.country.clone(),
        })
        .collect();

    let total = CITIES.len();
    let edge_count = edges.len();
    let json = serde_json::to_string_pretty(&edges).expect("serialize failed");
    fs::write("benches/edges.json", json).expect("write benches/edges.json failed");
    println!(
        "Scanned {total} cities: {edge_count} FuzzyFinder misses ({:.1}%) → benches/edges.json",
        edge_count as f64 / total as f64 * 100.0,
    );
}
