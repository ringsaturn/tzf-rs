//! Internal finder mechanisms shared by the public types: the materialized
//! int32 polygon finder (backing `DefaultFinder`) with its dense GRID
//! candidate index, and the FUZZY tile fast path.

use crate::tzb::{Error, ExpandedPolygon, Reader, TileId};
use geometry_rs::{I32Polygon, I32RaycastMode, Point, PolygonBuildOptions};
use std::collections::HashMap;

/// Minimum ring segment count for building the YStripes acceleration index;
/// smaller rings are scanned linearly. 32 matches the Go implementation
/// (`internal/geom.minIndexSegments`).
const RTREE_MIN_SEGMENTS: usize = 32;

const BUILD_OPTIONS: PolygonBuildOptions = PolygonBuildOptions {
    enable_rtree: false,
    enable_compressed_quad: false,
    enable_y_stripes: true,
    rtree_min_segments: RTREE_MIN_SEGMENTS,
};

/// One timezone's polygons.
pub(crate) struct Item {
    pub name: String,
    pub polys: Vec<I32Polygon>,
}

impl Item {
    fn contains_point(&self, p: Point) -> bool {
        // Timezone polygons tile the globe, so a query that lands exactly on
        // a shared border must belong to both neighbours rather than to
        // neither. The nautical zones make this easy to hit: their borders
        // sit on whole meridians (7.5°, 22.5°, …), which is exactly the kind
        // of coordinate people type by hand.
        self.polys
            .iter()
            .any(|poly| poly.contains_point_allow_on_edge(p))
    }
}

/// The dense 1°×1° GRID candidate index, copied out of the file so queries
/// probe a flat array instead of a hash map.
pub(crate) struct DenseGrid {
    lng_min: i32,
    lat_min: i32,
    lng_cells: u32,
    lat_cells: u32,
    words: Vec<u32>,
    cands: Vec<u16>,
}

impl DenseGrid {
    pub(crate) fn from_reader(reader: &Reader<'_>) -> Option<Self> {
        let (g, words, cands) = reader.grid_arrays()?;
        Some(Self {
            lng_min: i32::from(g.lng_min),
            lat_min: i32::from(g.lat_min),
            lng_cells: u32::from(g.lng_cells),
            lat_cells: u32::from(g.lat_cells),
            words,
            cands,
        })
    }

    /// The candidate range for (lng, lat): count 0 means no candidate covers
    /// the point. Offsets were bounds-checked at open.
    fn cell_range(&self, lng: f64, lat: f64) -> (u32, u32) {
        if !lng.is_finite()
            || !lat.is_finite()
            || !(-180.0..=180.0).contains(&lng)
            || !(-90.0..=90.0).contains(&lat)
        {
            return (0, 0);
        }
        #[allow(clippy::cast_possible_truncation)]
        let cx = lng.floor() as i64 - i64::from(self.lng_min);
        #[allow(clippy::cast_possible_truncation)]
        let cy = lat.floor() as i64 - i64::from(self.lat_min);
        if cx < 0 || cy < 0 || cx >= i64::from(self.lng_cells) || cy >= i64::from(self.lat_cells) {
            return (0, 0);
        }
        let word = self.words[(cy as u64 * u64::from(self.lng_cells) + cx as u64) as usize];
        (word & 0x0fff_ffff, word >> 28)
    }

    fn candidate(&self, off: u32) -> u32 {
        u32::from(self.cands[off as usize])
    }
}

/// The materialized point-in-polygon finder behind [`crate::DefaultFinder`].
pub(crate) struct PolyFinder {
    pub items: Vec<Item>,
    pub grid: Option<DenseGrid>,
    pub version: String,
}

impl PolyFinder {
    pub(crate) fn get_tz_name(&self, lng: f64, lat: f64) -> &str {
        if let Some(grid) = &self.grid {
            let (off, count) = grid.cell_range(lng, lat);
            if count == 0 {
                return "";
            }
            // Single-candidate short-circuit: skip PIP when there is only one
            // candidate and we are away from the antimeridian / pole edges.
            if count == 1 && lng > -179.0 && lng < 179.0 && lat > -89.0 && lat < 89.0 {
                return &self.items[grid.candidate(off) as usize].name;
            }
            let p = Point { x: lng, y: lat };
            for i in 0..count {
                let item = &self.items[grid.candidate(off + i) as usize];
                if item.contains_point(p) {
                    return &item.name;
                }
            }
            return "";
        }
        let p = Point { x: lng, y: lat };
        for item in &self.items {
            if item.contains_point(p) {
                return &item.name;
            }
        }
        ""
    }

    /// All matching timezone names, sorted lexicographically.
    pub(crate) fn get_tz_names(&self, lng: f64, lat: f64) -> Vec<&str> {
        let p = Point { x: lng, y: lat };
        let mut res: Vec<&str> = Vec::new();
        if let Some(grid) = &self.grid {
            let (off, count) = grid.cell_range(lng, lat);
            for i in 0..count {
                let item = &self.items[grid.candidate(off + i) as usize];
                if item.contains_point(p) {
                    res.push(&item.name);
                }
            }
        } else {
            for item in &self.items {
                if item.contains_point(p) {
                    res.push(&item.name);
                }
            }
        }
        res.sort_unstable();
        res
    }

    pub(crate) fn timezonenames(&self) -> Vec<&str> {
        self.items.iter().map(|item| item.name.as_str()).collect()
    }
}

/// Builds the finder items. Assembly cost is dominated by the per-ring
/// YStripes build and every timezone is independent, so the work fans out
/// across the CPUs; used by both the `.tzb` expansion loader and the `.tzm`
/// loader.
pub(crate) fn assemble_items(names: Vec<String>, polygons: Vec<Vec<ExpandedPolygon>>) -> Vec<Item> {
    let mut pairs: Vec<(String, Vec<ExpandedPolygon>)> = names.into_iter().zip(polygons).collect();
    let threads = std::thread::available_parallelism()
        .map(std::num::NonZero::get)
        .unwrap_or(1)
        .min(pairs.len().max(1));
    if threads <= 1 {
        return pairs
            .drain(..)
            .map(|(name, polys)| build_item(name, polys))
            .collect();
    }
    let chunk_size = pairs.len().div_ceil(threads);
    std::thread::scope(|scope| {
        let handles: Vec<_> = pairs
            .chunks_mut(chunk_size)
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter_mut()
                        .map(|slot| {
                            let (name, polys) = std::mem::take(slot);
                            build_item(name, polys)
                        })
                        .collect::<Vec<Item>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("item assembly thread panicked"))
            .collect()
    })
}

fn build_item(name: String, polys: Vec<ExpandedPolygon>) -> Item {
    let polys = polys
        .into_iter()
        .map(|poly| {
            // The loaders produce open rings; geometry-rs stores rings closed
            // (first point repeated at the end) and does not close them
            // itself — without the closing segment the raycast leaks.
            I32Polygon::new_with_options(
                close_ring(poly.exterior),
                poly.holes.into_iter().map(close_ring).collect(),
                1e5,
                I32RaycastMode::Float,
                Some(BUILD_OPTIONS),
            )
        })
        .collect();
    Item { name, polys }
}

fn close_ring(mut ring: Vec<geometry_rs::I32Point>) -> Vec<geometry_rs::I32Point> {
    if let Some(&first) = ring.first() {
        ring.push(first);
    }
    ring
}

/// Most tiles belong to exactly one timezone, so store that index inline and
/// only heap-allocate for boundary tiles that straddle multiple timezones.
enum TileEntry {
    One(u16),
    Many(Box<[u16]>),
}

impl TileEntry {
    fn first(&self) -> u16 {
        match self {
            Self::One(idx) => *idx,
            Self::Many(idxs) => idxs[0],
        }
    }
}

/// The preindex tile fast path rebuilt out of a file's FUZZY section. Not a
/// public mechanism in v2: a query it cannot answer falls through to the
/// polygon finder.
pub(crate) struct FuzzyIndex {
    idx_zoom: u8,
    agg_zoom: u8,
    tiles: HashMap<u64, TileEntry>,
}

impl FuzzyIndex {
    /// Rebuilds the preindex hash map from a file's FUZZY section: one pass
    /// over the sorted tile keys. Returns `None` when the file carries no
    /// FUZZY section.
    pub(crate) fn from_reader(reader: &Reader<'_>) -> Result<Option<Self>, Error> {
        let Some((idx_zoom, agg_zoom)) = reader.fuzzy_zooms() else {
            return Ok(None);
        };
        let entries = reader.fuzzy_entries()?;
        let mut tiles = HashMap::with_capacity(entries.len());
        for (key, indices) in entries {
            let entry = if indices.len() == 1 {
                TileEntry::One(indices[0])
            } else {
                TileEntry::Many(indices.into_boxed_slice())
            };
            tiles.insert(key, entry);
        }
        Ok(Some(Self {
            idx_zoom,
            agg_zoom,
            tiles,
        }))
    }

    /// The tile answer for (lng, lat), coarsest zoom first; multi-name tiles
    /// resolve to the group's first entry (first-listed wins). `None` means
    /// no tile covers the point and the caller falls back to polygons.
    pub(crate) fn get(&self, lng: f64, lat: f64) -> Option<u16> {
        if !lng.is_finite()
            || !lat.is_finite()
            || !(-180.0..=180.0).contains(&lng)
            || !(-90.0..=90.0).contains(&lat)
        {
            return None;
        }
        let tile = TileId::new(lng, lat, u32::from(self.idx_zoom));
        for z in self.agg_zoom..=self.idx_zoom {
            let key = tile.shift(self.idx_zoom - z).0;
            if let Some(entry) = self.tiles.get(&key) {
                return Some(entry.first());
            }
        }
        None
    }
}
