use std::env;
use tzf_rel::load_reduced;
use tzf_rs::{DefaultFinder, Finder, FinderOptions, pbgen};

fn parse_mode(mode: &str) -> FinderOptions {
    match mode {
        "ystripes" => FinderOptions::y_stripes(),
        "noindex" => FinderOptions::no_index(),
        _ => FinderOptions::y_stripes(),
    }
}

fn main() {
    let target = env::args().nth(1).unwrap_or_else(|| "finder".to_string());
    let mode = env::args().nth(2).unwrap_or_else(|| "ystripes".to_string());

    match target.as_str() {
        "default" => {
            let options = parse_mode(&mode);
            let default_finder = DefaultFinder::new_with_options(options);
            println!("{}", default_finder.timezonenames().len());
        }
        _ => {
            let options = parse_mode(&mode);
            let tzs = pbgen::Timezones::try_from(load_reduced()).unwrap_or_default();
            let finder = Finder::from_pb_with_options(tzs, options);
            println!("{}", finder.timezonenames().len());
        }
    }
}
