use std::time::Instant;
use tzf_rs::{DefaultFinder, EmbeddedFinder};

fn main() {
    let started_at = Instant::now();
    let default_finder = DefaultFinder::new();
    println!("DefaultFinder init: {:.3?}", started_at.elapsed());

    let started_at = Instant::now();
    let embedded_finder = EmbeddedFinder::new();
    println!("EmbeddedFinder init: {:.3?}", started_at.elapsed());

    // Please note coords are lng-lat.
    println!("{:?}", default_finder.get_tz_name(116.3883, 39.9289));
    println!("{:?}", default_finder.get_tz_names(87.4160, 44.0400));
    println!("{:?}", embedded_finder.get_tz_name(139.767125, 35.681236));
    println!("data version: {}", default_finder.data_version());
}
