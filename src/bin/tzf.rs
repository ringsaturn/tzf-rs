#![cfg(feature = "clap")]

use clap::Parser;
use tzf_rs::DefaultFinder;

#[derive(Parser, Debug)]
#[command(name = "tzf")]
struct Cli {
    /// longitude
    #[arg(long, allow_negative_numbers(true), alias("lon"))]
    lng: f64,

    /// latitude
    #[arg(long, allow_negative_numbers(true))]
    lat: f64,
}

pub fn main() {
    let cli = Cli::parse();
    let finder = DefaultFinder::new();
    let tz_name = finder.get_tz_name(cli.lng, cli.lat);
    println!("{tz_name:?}");
}
