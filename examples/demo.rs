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
    let y_stripes_finder = build_finder("y_stripes", Some(FinderOptions::y_stripes()));

    // Please note coords are lng-lat.
    println!("{:?}", default_finder.get_tz_name(116.3883, 39.9289));
    println!("{:?}", default_finder.get_tz_names(87.4160, 44.0400));
    println!("{:?}", y_stripes_finder.get_tz_name(139.767125, 35.681236));
}
