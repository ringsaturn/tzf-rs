//! Internal finder mechanisms shared by the public types: the materialized
//! int32 polygon finder (backing `DefaultFinder`) with its dense GRID
//! candidate index, and the FUZZY tile fast path.

use crate::tzb::{Error, ExpandedPolygon, Reader, TileId};
use geometry_rs::{I32Polygon, I32RaycastMode, Point, PolygonBuildOptions};
use rustc_hash::{FxBuildHasher, FxHashMap};
#[cfg(feature = "export-geojson")]
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

    #[cfg(feature = "export-geojson")]
    fn indices(&self) -> &[u16] {
        match self {
            Self::One(idx) => std::slice::from_ref(idx),
            Self::Many(idxs) => idxs,
        }
    }
}

/// The preindex tile fast path rebuilt out of a file's FUZZY section. Not a
/// public mechanism in v2: a query it cannot answer falls through to the
/// polygon finder.
pub(crate) struct FuzzyIndex {
    idx_zoom: u8,
    agg_zoom: u8,
    /// Keyed by tile id. The map is built once from the dataset and only read
    /// afterwards, so a fast non-keyed hasher is safe here: query coordinates
    /// cannot insert keys, so they cannot degrade the table.
    tiles: FxHashMap<u64, TileEntry>,
    /// Per 1°×1° cell (`ZOOM_CELLS_LNG` × `ZOOM_CELLS_LAT`, row-major from
    /// lat -90, lng -180), the coarsest and finest zoom of any tile touching
    /// the cell, packed as `zmin | zmax << 8`; [`NO_TILES`] when none does.
    /// A lookup probes only that zoom range: a tile covering a point always
    /// touches the point's cell, so the skipped zooms can never match, and
    /// the probe order (coarsest first) is unchanged.
    zoom_ranges: Box<[u16]>,
}

const ZOOM_CELLS_LNG: usize = 360;
const ZOOM_CELLS_LAT: usize = 180;
const NO_TILES: u16 = u16::MAX;

/// The 1°×1° cell holding (lng, lat); the caller has range-checked both.
/// `lng = 180` and `lat = 90` fold into the last column / row.
#[allow(clippy::cast_sign_loss)]
fn zoom_cell(lng: f64, lat: f64) -> usize {
    let cx = ((lng + 180.0) as usize).min(ZOOM_CELLS_LNG - 1);
    let cy = ((lat + 90.0) as usize).min(ZOOM_CELLS_LAT - 1);
    cy * ZOOM_CELLS_LNG + cx
}

/// Builds [`FuzzyIndex::zoom_ranges`] from the tile keys.
#[allow(clippy::cast_sign_loss)]
fn build_zoom_ranges(keys: &[u64]) -> Box<[u16]> {
    // Widen each tile by a hair so float error in the projection can only
    // mark extra cells, never miss one.
    const MARGIN: f64 = 1e-7;

    // Row edge latitudes per zoom (inverse of the Web-Mercator row in
    // `tile_xy`), computed once: a zoom has 2^z + 1 row edges but holds far
    // more tiles than that.
    let max_zoom = keys.iter().map(|&k| TileId(k).xyz().2).max().unwrap_or(0);
    let row_edges: Vec<Vec<f64>> = (0..=max_zoom)
        .map(|z| {
            let n = f64::from(1u32 << z);
            (0..=(1u32 << z))
                .map(|y| {
                    (std::f64::consts::PI * (1.0 - 2.0 * f64::from(y) / n))
                        .sinh()
                        .atan()
                        .to_degrees()
                })
                .collect()
        })
        .collect();

    let cells = ZOOM_CELLS_LNG * ZOOM_CELLS_LAT;
    let mut zmin = vec![u8::MAX; cells];
    let mut zmax = vec![0u8; cells];
    for &key in keys {
        let (x, y, z) = TileId(key).xyz();
        let rows = 1u32 << z;
        let n = f64::from(rows);
        let edges = &row_edges[usize::from(z)];
        let lng0 = f64::from(x) / n * 360.0 - 180.0 - MARGIN;
        let lng1 = f64::from(x + 1) / n * 360.0 - 180.0 + MARGIN;
        // `tile_xy` clamps latitudes beyond ±85.0511° into the first / last
        // row, so those rows reach the poles.
        let lat_top = if y == 0 {
            90.0
        } else {
            edges[y as usize] + MARGIN
        };
        let lat_bottom = if y + 1 >= rows {
            -90.0
        } else {
            edges[y as usize + 1] - MARGIN
        };
        let cx0 = ((lng0 + 180.0).floor().max(0.0) as usize).min(ZOOM_CELLS_LNG - 1);
        let cx1 = ((lng1 + 180.0).floor().max(0.0) as usize).min(ZOOM_CELLS_LNG - 1);
        let cy0 = ((lat_bottom + 90.0).floor().max(0.0) as usize).min(ZOOM_CELLS_LAT - 1);
        let cy1 = ((lat_top + 90.0).floor().max(0.0) as usize).min(ZOOM_CELLS_LAT - 1);
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                let c = cy * ZOOM_CELLS_LNG + cx;
                zmin[c] = zmin[c].min(z);
                zmax[c] = zmax[c].max(z);
            }
        }
    }
    zmin.iter()
        .zip(&zmax)
        .map(|(&lo, &hi)| {
            if lo == u8::MAX {
                NO_TILES
            } else {
                u16::from(lo) | u16::from(hi) << 8
            }
        })
        .collect()
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
        let keys: Vec<u64> = entries.iter().map(|(key, _)| *key).collect();
        let zoom_ranges = build_zoom_ranges(&keys);
        let mut tiles = FxHashMap::with_capacity_and_hasher(entries.len(), FxBuildHasher);
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
            zoom_ranges,
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
        let range = self.zoom_ranges[zoom_cell(lng, lat)];
        if range == NO_TILES {
            return None;
        }
        #[allow(clippy::cast_possible_truncation)]
        let (lo, hi) = ((range & 0xff) as u8, (range >> 8) as u8);
        let tile = TileId::new(lng, lat, u32::from(self.idx_zoom));
        for z in lo.max(self.agg_zoom)..=hi.min(self.idx_zoom) {
            let key = tile.shift(self.idx_zoom - z).0;
            if let Some(entry) = self.tiles.get(&key) {
                return Some(entry.first());
            }
        }
        None
    }

    /// All tile keys carrying any of the given timezone indices, sorted
    /// ascending — the zoom lives in the key's high bits, so coarser tiles
    /// come first. Deterministic despite the hash-map storage.
    #[cfg(feature = "export-geojson")]
    pub(crate) fn tile_keys_for(&self, indices: &[u16]) -> Vec<u64> {
        let mut keys: Vec<u64> = self
            .tiles
            .iter()
            .filter(|(_, entry)| entry.indices().iter().any(|i| indices.contains(i)))
            .map(|(key, _)| *key)
            .collect();
        keys.sort_unstable();
        keys
    }

    /// One pass over all tiles, grouping keys by the timezone index they
    /// carry (a boundary tile lands in every group it names); each group
    /// sorted ascending.
    #[cfg(feature = "export-geojson")]
    pub(crate) fn tile_keys_grouped(&self) -> HashMap<u16, Vec<u64>> {
        let mut grouped: HashMap<u16, Vec<u64>> = HashMap::new();
        for (key, entry) in &self.tiles {
            for &idx in entry.indices() {
                grouped.entry(idx).or_default().push(*key);
            }
        }
        for keys in grouped.values_mut() {
            keys.sort_unstable();
        }
        grouped
    }
}

#[cfg(all(test, any(feature = "bundled", feature = "full")))]
mod tests {
    use super::*;

    impl FuzzyIndex {
        /// The pre-gate lookup: every zoom from `agg_zoom` to `idx_zoom`.
        fn get_all_zooms(&self, lng: f64, lat: f64) -> Option<u16> {
            if !lng.is_finite()
                || !lat.is_finite()
                || !(-180.0..=180.0).contains(&lng)
                || !(-90.0..=90.0).contains(&lat)
            {
                return None;
            }
            let tile = TileId::new(lng, lat, u32::from(self.idx_zoom));
            (self.agg_zoom..=self.idx_zoom)
                .find_map(|z| self.tiles.get(&tile.shift(self.idx_zoom - z).0))
                .map(TileEntry::first)
        }
    }

    /// Deterministic xorshift in [0, 1).
    struct Rng(u64);
    impl Rng {
        #[allow(clippy::cast_precision_loss)]
        fn next(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    #[test]
    fn zoom_gate_matches_probing_every_zoom() {
        check_zoom_gate(crate::load_lite_tzb());
    }

    #[cfg(feature = "full")]
    #[test]
    fn zoom_gate_matches_probing_every_zoom_full() {
        check_zoom_gate(tzf_dist_git::load_full_tzb());
    }

    fn check_zoom_gate(data: &[u8]) {
        let reader = Reader::open(std::borrow::Cow::Borrowed(data)).unwrap();
        let fuzzy = FuzzyIndex::from_reader(&reader).unwrap().unwrap();

        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let mut points: Vec<(f64, f64)> = (0..300_000)
            .map(|_| (rng.next() * 360.0 - 180.0, rng.next() * 180.0 - 90.0))
            .collect();
        // polar caps, where the Mercator rows are clamped
        points.extend((0..50_000).map(|_| (rng.next() * 360.0 - 180.0, 80.0 + rng.next() * 10.0)));
        points.extend((0..50_000).map(|_| (rng.next() * 360.0 - 180.0, -90.0 + rng.next() * 10.0)));
        // points exactly on 1° cell edges and the domain edges
        for lat in (-90..=90).step_by(5) {
            for lng in (-180..=180).step_by(5) {
                points.push((f64::from(lng), f64::from(lat)));
            }
        }
        for lat in [-85.0511, 85.0511, -85.051_13, 85.051_13] {
            for lng in [-180.0, -0.0, 0.0, 179.999_999, 180.0] {
                points.push((lng, lat));
            }
        }

        let mut hits = 0;
        for &(lng, lat) in &points {
            let gated = fuzzy.get(lng, lat);
            assert_eq!(gated, fuzzy.get_all_zooms(lng, lat), "at ({lng}, {lat})");
            hits += usize::from(gated.is_some());
        }
        assert!(hits > 0, "the sample never reached the fuzzy index");
    }
}
