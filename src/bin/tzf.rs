#![cfg(feature = "clap")]

use clap::Parser;
use tzf_rs::DefaultFinder;

#[derive(Parser, Debug)]
#[command(name = "tzf")]
struct Cli {
    /// longitude
    #[arg(long)]
    lng: f64,

    /// latitude
    #[arg(long)]
    lat: f64,
}

pub fn main() {
    let cli = Cli::parse();
    let finder = DefaultFinder::new();
    println!("{:?}", finder.get_tz_name(cli.lng, cli.lat));
}
