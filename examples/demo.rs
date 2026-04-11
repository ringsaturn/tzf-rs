use std::time::{Duration, Instant};
use tzf_rs::{DefaultFinder, FinderOptions};

fn build_finder(label: &str, options: Option<FinderOptions>) -> DefaultFinder {
    let started_at = Instant::now();
    let finder = match options {
        Some(options) => DefaultFinder::new_with_options(options),
        None => DefaultFinder::new(),
    };
    print_init_time(label, started_at.elapsed());
    finder
}

fn print_init_time(label: &str, elapsed: Duration) {
    println!("{label} init: {:.3?}", elapsed);
}

fn main() {
    let default_finder = build_finder("default", None);
    let rtree_finder = build_finder("rtree", Some(FinderOptions::rtree()));
    let quad_tree_finder = build_finder("quad_tree", Some(FinderOptions::quad_tree()));

    // Please note coords are lng-lat.
    println!("{:?}", default_finder.get_tz_name(116.3883, 39.9289));
    println!("{:?}", default_finder.get_tz_names(87.4160, 44.0400));
    println!("{:?}", rtree_finder.get_tz_name(139.767125, 35.681236));
    println!("{:?}", quad_tree_finder.get_tz_name(139.767125, 35.681236));
}
