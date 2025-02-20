#![cfg(feature = "clap")]

use clap::{Args, Parser, ValueEnum};
use std::error::Error;
use std::io::{self, BufRead, Write};
use tzf_rs::DefaultFinder;

#[derive(Parser, Debug)]
#[command(name = "tzf")]
struct Cli {
    #[command(flatten)]
    params: Option<Params>,

    /// Read multiple coordinates from stdin in given order
    #[arg(long, conflicts_with("Params"))]
    stdin_order: Option<StdinOrder>,
}

#[derive(Args, Debug)]
struct Params {
    /// Longitude
    #[arg(long, allow_negative_numbers(true), alias("lon"))]
    lng: f64,

    /// Latitude
    #[arg(long, allow_negative_numbers(true))]
    lat: f64,
}

#[derive(Clone, Debug, ValueEnum)]
enum StdinOrder {
    #[value(alias("lon-lat"))]
    LngLat,
    #[value(alias("lat-lon"))]
    LatLng,
}

fn is_delimiter(c: char) -> bool {
    matches!(c, ' ' | '\t' | ',' | ';')
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let finder = DefaultFinder::new();
    if let Some(params) = cli.params {
        println!("{:?}", finder.get_tz_name(params.lng, params.lat));
    } else if let Some(stdin_order) = cli.stdin_order {
        let (mut stdin, mut stdout) = (io::stdin().lock(), io::stdout().lock());
        let mut line = String::new();
        while stdin.read_line(&mut line)? != 0 && line.ends_with("\n") {
            let mut iter = line.chars().skip(1);
            let i = 1 + iter.position(is_delimiter).expect("Missing delimiter");
            let j = i
                + 1
                + iter
                    .position(|c| !is_delimiter(c))
                    .expect("Missing second coordinate");
            let k = line.len() - if line.ends_with("\r\n") { 2 } else { 1 };
            let (a, b) = (line[0..i].parse::<f64>()?, line[j..k].parse::<f64>()?);
            let (lng, lat) = match stdin_order {
                StdinOrder::LngLat => (a, b),
                StdinOrder::LatLng => (b, a),
            };
            writeln!(stdout, "{:?}", finder.get_tz_name(lng, lat))?;
            line.clear();
        }
    }
    Ok(())
}
