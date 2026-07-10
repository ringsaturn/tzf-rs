//! Measure live heap bytes held by a Finder after construction, using a
//! counting global allocator. Compare `ystripes` vs `noindex` to isolate the
//! memory cost of the Y-stripes polygon index.
//!
//! Usage: cargo run --release --example index_memory_probe_alloc -- [ystripes|noindex]

use std::alloc::{GlobalAlloc, Layout, System};
use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};
use tzf_dist::load_topology_compress_topo;
use tzf_rs::{Finder, FinderOptions, pbgen};

struct CountingAlloc;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn on_alloc(size: usize) {
    let live = LIVE.fetch_add(size, Ordering::Relaxed) + size;
    PEAK.fetch_max(live, Ordering::Relaxed);
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            on_alloc(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            if new_size >= layout.size() {
                on_alloc(new_size - layout.size());
            } else {
                LIVE.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }
        new_ptr
    }
}

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

fn main() {
    let mode = env::args().nth(1).unwrap_or_else(|| "ystripes".to_string());
    let options = match mode.as_str() {
        "noindex" => FinderOptions::no_index(),
        _ => FinderOptions::y_stripes(),
    };

    let before = LIVE.load(Ordering::Relaxed);
    let tzs =
        pbgen::CompressedTopoTimezones::try_from(load_topology_compress_topo()).unwrap_or_default();
    let finder = Finder::from_compressed_topo_with_options(tzs, options);
    let after = LIVE.load(Ordering::Relaxed);

    println!("mode: {mode}");
    println!("finder live heap bytes: {}", after - before);
    println!("peak heap bytes: {}", PEAK.load(Ordering::Relaxed));
    println!(
        "sanity lookup Beijing: {}",
        finder.get_tz_name(116.3883, 39.9289)
    );
}
