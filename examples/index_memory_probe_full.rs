use tzf_rs::DefaultFinder;

fn main() {
    let finder = DefaultFinder::new_full();
    println!("{}", finder.timezonenames().len());
}
