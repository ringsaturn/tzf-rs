use std::env;
use tzf_dist_git::load_compress_topo;
use tzf_rs::{DefaultFinder, Finder, FinderOptions, pbgen};

fn parse_mode(mode: &str) -> FinderOptions {
    match mode {
        "noindex" => FinderOptions::no_index(),
        _ => FinderOptions::y_stripes(),
    }
}

fn main() {
    let target = env::args()
        .nth(1)
        .unwrap_or_else(|| "default_full".to_string());
    let mode = env::args().nth(2).unwrap_or_else(|| "ystripes".to_string());
    let options = parse_mode(&mode);

    match target.as_str() {
        "finder_full" => {
            let tzs =
                pbgen::CompressedTopoTimezones::try_from(load_compress_topo()).unwrap_or_default();
            let finder = Finder::from_compressed_topo_with_options(tzs, options);
            println!("{}", finder.timezonenames().len());
        }
        _ => {
            let finder = DefaultFinder::new_full_with_options(options);
            println!("{}", finder.timezonenames().len());
        }
    }
}
