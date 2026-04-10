use lazy_static::lazy_static;
use tzf_rs::{DefaultFinder, IndexMode};

lazy_static! {
    static ref FINDER: DefaultFinder = DefaultFinder::new();
    static ref FINDER_RTREE: DefaultFinder = DefaultFinder::new_with_index(IndexMode::RTree);
    static ref FINDER_QUAD: DefaultFinder = DefaultFinder::new_with_index(IndexMode::QuadTree);
}

fn main() {
    // Please note coords are lng-lat.
    print!("{:?}\n", FINDER.get_tz_name(116.3883, 39.9289));
    print!("{:?}\n", FINDER.get_tz_names(116.3883, 39.9289));

    print!("{:?}\n", FINDER_RTREE.get_tz_name(116.3883, 39.9289));
    print!("{:?}\n", FINDER_RTREE.get_tz_names(116.3883, 39.9289));
}
