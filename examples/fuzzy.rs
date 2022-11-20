use std::time::Instant;
use tzf_rs::FuzzyFinder;

fn main() {
    let finder = FuzzyFinder::new();

    let now = Instant::now();

    print!("tz={:?}\n", finder.get_tz_name(116.3883, 39.9289));

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}\n", elapsed);
}
