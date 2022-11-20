use std::time::Instant;
use tzf_rs::DefaultFinder;

fn main() {
    let finder = DefaultFinder::new();

    let now = Instant::now();

    print!("{:?}\n", finder.get_tz_name(116.3883, 39.9289));

    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}\n", elapsed);
}
