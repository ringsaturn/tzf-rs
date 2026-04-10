use std::env;
use tzf_rel::load_reduced;
use tzf_rs::{DefaultFinder, Finder, IndexMode, pbgen};

fn parse_mode(mode: &str) -> IndexMode {
    match mode {
        "rtree" => IndexMode::RTree,
        "noindex" => IndexMode::NoIndex,
        _ => IndexMode::QuadTree,
    }
}

fn main() {
    let target = env::args().nth(1).unwrap_or_else(|| "finder".to_string());
    let mode = env::args().nth(2).unwrap_or_else(|| "quad".to_string());

    match target.as_str() {
        "default" => {
            let index_mode = parse_mode(&mode);
            let default_finder = DefaultFinder::new_with_index(index_mode);
            println!("{}", default_finder.timezonenames().len());
        }
        _ => {
            let index_mode = parse_mode(&mode);
            let tzs = pbgen::Timezones::try_from(load_reduced()).unwrap_or_default();
            let finder = Finder::from_pb_with_index(tzs, index_mode);
            println!("{}", finder.timezonenames().len());
        }
    }
}
