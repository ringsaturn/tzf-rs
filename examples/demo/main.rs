use std::time::Instant;
use tzf_rs::Finder;

fn main() {
    let finder = Finder::new_default();

    let now = Instant::now();

    print!("{:?}", finder.get_tz_name(116.3883, 39.9289));

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
}
