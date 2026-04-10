use geometry_rs::PolygonBuildOptions;
use std::env;
use tzf_rel::load_reduced;
use tzf_rs::{DefaultFinder, Finder, pbgen};

fn parse_mode(mode: &str) -> PolygonBuildOptions {
    match mode {
        "rtree" => PolygonBuildOptions {
            enable_rtree: true,
            enable_compressed_quad: false,
            rtree_min_segments: 64,
        },
        "quad" => PolygonBuildOptions {
            enable_rtree: false,
            enable_compressed_quad: true,
            rtree_min_segments: 64,
        },
        _ => PolygonBuildOptions {
            enable_rtree: false,
            enable_compressed_quad: false,
            rtree_min_segments: 64,
        },
    }
}

fn main() {
    let target = env::args().nth(1).unwrap_or_else(|| "finder".to_string());
    let mode = env::args().nth(2).unwrap_or_else(|| "noindex".to_string());

    match target.as_str() {
        "default" => {
            let options = parse_mode(&mode);
            let default_finder = DefaultFinder::new_with_index_options(options);
            println!("{}", default_finder.timezonenames().len());
        }
        _ => {
            let options = parse_mode(&mode);
            let tzs = pbgen::Timezones::try_from(load_reduced()).unwrap_or_default();
            let finder = Finder::from_pb_with_index_options(tzs, options);
            println!("{}", finder.timezonenames().len());
        }
    }
}
