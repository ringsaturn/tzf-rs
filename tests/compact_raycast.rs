use serde::Deserialize;
use std::fs;
use tzf_dist::load_topology_compress_topo;
use tzf_rs::{Finder, FinderOptions, pbgen};

#[derive(Deserialize)]
struct Coordinate {
    lng: f64,
    lat: f64,
}

#[test]
fn integer_and_float_raycasts_match_on_edge_cities() {
    let load = || {
        pbgen::CompressedTopoTimezones::try_from(load_topology_compress_topo()).unwrap_or_default()
    };
    let float =
        Finder::from_compressed_topo_with_options(load(), FinderOptions::no_index_float_raycast());
    let integer = Finder::from_compressed_topo_with_options(
        load(),
        FinderOptions::no_index_integer_raycast(),
    );
    let coordinates: Vec<Coordinate> = serde_json::from_str(
        &fs::read_to_string("benches/edges.json").expect("missing edge benchmark data"),
    )
    .expect("invalid edge benchmark data");
    for coordinate in coordinates {
        assert_eq!(
            float.get_tz_names(coordinate.lng, coordinate.lat),
            integer.get_tz_names(coordinate.lng, coordinate.lat),
            "raycast mismatch at {}, {}",
            coordinate.lng,
            coordinate.lat,
        );
    }
}
