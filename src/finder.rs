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
    /// Index into `items` of the first timezone covering the point.
    pub(crate) fn get_index(&self, lng: f64, lat: f64) -> Option<usize> {
        if let Some(grid) = &self.grid {
            let (off, count) = grid.cell_range(lng, lat);
            if count == 0 {
                return None;
            }
            // Single-candidate short-circuit: skip PIP when there is only one
            // candidate and we are away from the antimeridian / pole edges.
            if count == 1 && lng > -179.0 && lng < 179.0 && lat > -89.0 && lat < 89.0 {
                return Some(grid.candidate(off) as usize);
            }
            let p = Point { x: lng, y: lat };
            return (0..count)
                .map(|i| grid.candidate(off + i) as usize)
                .find(|&i| self.items[i].contains_point(p));
        }
        let p = Point { x: lng, y: lat };
        self.items.iter().position(|item| item.contains_point(p))
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
#[cfg(feature = "export-geojson")]
enum TileEntry {
    One(u16),
    Many(Box<[u16]>),
}

#[cfg(feature = "export-geojson")]
impl TileEntry {
    fn indices(&self) -> &[u16] {
        match self {
            Self::One(idx) => std::slice::from_ref(idx),
            Self::Many(idxs) => idxs,
        }
    }
}

/// Side of a [`FuzzyIndex`] block: each block level refines a node by two
/// zooms, into `BLOCK_SIDE` × `BLOCK_SIDE` children.
const BLOCK_SIDE: u32 = 4;
const BLOCK_CELLS: usize = (BLOCK_SIDE * BLOCK_SIDE) as usize;
/// The finest root zoom: 4^8 = 65,536 nodes (128 KB).
const MAX_ROOT_ZOOM: u8 = 8;
/// A grid node: a timezone index, a block pointer, or [`NO_TILE`].
type Node = u16;
/// A node no tile covers.
const NO_TILE: Node = Node::MAX;
/// The wider node used while building, before identical blocks are merged.
type BuildNode = u32;
type Nodes = Box<[Node]>;
const BUILD_NO_TILE: BuildNode = BuildNode::MAX;

/// The preindex tile fast path rebuilt out of a file's FUZZY section, as a
/// flat tile grid. Not a public mechanism in v2: a query it cannot answer
/// falls through to the polygon finder.
///
/// The root is a dense `2^root_zoom` × `2^root_zoom` array of nodes. Below
/// it, every two zooms form one block level down to `leaf_zoom`, the finest
/// zoom any tile has: a node whose area holds finer tiles points to a block
/// of 4 × 4 child nodes. A node is a timezone index (`< block_base`), a block
/// pointer (`block_base + block`), or [`NO_TILE`]. A lookup is one root read
/// plus one read per block level, replacing a hash probe per zoom.
///
/// Answers are those of probing every zoom from `agg_zoom` to `idx_zoom`,
/// coarsest first, with multi-name tiles resolving to their first name: the
/// grid is built finest tiles first, so a coarser tile overwrites the area of
/// any finer one.
pub(crate) struct FuzzyIndex {
    idx_zoom: u8,
    root_zoom: u8,
    leaf_zoom: u8,
    block_base: Node,
    root: Nodes,
    blocks: Nodes,
    /// Sorted ascending by key: the zoom lives in the key's high bits, so
    /// coarser tiles come first.
    #[cfg(feature = "export-geojson")]
    tiles: Vec<(u64, TileEntry)>,
}

impl FuzzyIndex {
    /// Rebuilds the preindex tile grid from a file's FUZZY section. Returns
    /// `None` when the file carries no FUZZY section.
    pub(crate) fn from_reader(reader: &Reader<'_>) -> Result<Option<Self>, Error> {
        let Some((idx_zoom, agg_zoom)) = reader.fuzzy_zooms() else {
            return Ok(None);
        };
        let entries = reader.fuzzy_entries()?;
        let block_base = Node::try_from(reader.timezone_count())
            .ok()
            .filter(|&n| n < NO_TILE)
            .map(BuildNode::from)
            .ok_or(Error::Malformed("too many timezones for the FUZZY grid"))?;

        // Only zooms in agg_zoom..=idx_zoom are ever probed.
        let mut tiles: Vec<(u32, u32, u8, BuildNode)> = entries
            .iter()
            .filter_map(|(key, indices)| {
                let (x, y, z) = TileId(*key).xyz();
                (agg_zoom..=idx_zoom)
                    .contains(&z)
                    .then(|| (x, y, z, BuildNode::from(indices[0])))
            })
            .collect();
        let leaf_zoom = tiles.iter().map(|t| t.2).max().unwrap_or(agg_zoom);
        // Block levels sit every two zooms above the leaf.
        let root_zoom = if leaf_zoom <= MAX_ROOT_ZOOM {
            leaf_zoom
        } else {
            MAX_ROOT_ZOOM - (leaf_zoom - MAX_ROOT_ZOOM) % 2
        };

        // Finest first: a coarser tile then overwrites any finer one under it.
        tiles.sort_by_key(|t| std::cmp::Reverse(t.2));
        let side = 1usize << root_zoom;
        let mut root = vec![BUILD_NO_TILE; side * side];
        let mut blocks: Vec<BuildNode> = Vec::new();
        for (x, y, z, value) in tiles {
            if z <= root_zoom {
                let span = 1u32 << (root_zoom - z);
                for ry in y * span..(y + 1) * span {
                    let row = ry as usize * side;
                    root[row + (x * span) as usize..row + ((x + 1) * span) as usize].fill(value);
                }
                continue;
            }
            // The block level holding z: the shallowest level at or below it.
            let level = root_zoom + (z - root_zoom).div_ceil(2) * 2;
            // Its top-left cell at that level and the cells it spans there.
            let span = 1u32 << (level - z);
            let (lx, ly) = (x * span, y * span);
            let shift = level - root_zoom;
            // The node being descended through: a root position, then a
            // position in `blocks`.
            let mut at_root = true;
            let mut pos = (ly >> shift) as usize * side + (lx >> shift) as usize;
            let mut depth = root_zoom;
            loop {
                let mut node = if at_root { root[pos] } else { blocks[pos] };
                if node == BUILD_NO_TILE {
                    // Descend into a fresh block.
                    node = BuildNode::try_from(blocks.len() / BLOCK_CELLS)
                        .ok()
                        .and_then(|b| b.checked_add(block_base))
                        .filter(|&v| v < BUILD_NO_TILE)
                        .ok_or(Error::Malformed("FUZZY grid exceeds its node range"))?;
                    if at_root {
                        root[pos] = node
                    } else {
                        blocks[pos] = node
                    }
                    blocks.extend([BUILD_NO_TILE; BLOCK_CELLS]);
                } else if node < block_base {
                    // Already covered by a coarser tile, which wins.
                    break;
                }
                let base = (node - block_base) as usize * BLOCK_CELLS;
                depth += 2;
                if depth == level {
                    // span ≤ 2 here, and an aligned span never crosses the block.
                    for cy in ly..ly + span {
                        for cx in lx..lx + span {
                            blocks[base + child_slot(cx, cy, 0)] = value;
                        }
                    }
                    break;
                }
                at_root = false;
                pos = base + child_slot(lx, ly, level - depth);
            }
        }

        let (root, blocks) = compact(&root, &blocks, block_base)?;
        #[allow(clippy::cast_possible_truncation)] // checked < NO_TILE above
        let block_base = block_base as Node;

        Ok(Some(Self {
            idx_zoom,
            root_zoom,
            leaf_zoom,
            block_base,
            root,
            blocks,
            #[cfg(feature = "export-geojson")]
            tiles: entries
                .into_iter()
                .map(|(key, indices)| {
                    let entry = if indices.len() == 1 {
                        TileEntry::One(indices[0])
                    } else {
                        TileEntry::Many(indices.into_boxed_slice())
                    };
                    (key, entry)
                })
                .collect(),
        }))
    }

    /// The tile answer for (lng, lat), coarsest zoom first; multi-name tiles
    /// resolve to the group's first entry (first-listed wins). `None` means
    /// no tile covers the point and the caller falls back to polygons.
    #[inline]
    pub(crate) fn get(&self, lng: f64, lat: f64) -> Option<u16> {
        // NaN fails every comparison, so it is rejected here too.
        if !((-180.0..=180.0).contains(&lng) && (-90.0..=90.0).contains(&lat)) {
            return None;
        }
        // Derived from the idx_zoom tile exactly as the per-zoom probe did,
        // so float rounding at tile edges cannot change an answer.
        let (x, y, _) = TileId::new(lng, lat, u32::from(self.idx_zoom))
            .shift(self.idx_zoom - self.leaf_zoom)
            .xyz();
        let shift = self.leaf_zoom - self.root_zoom;
        let side_bits = self.root_zoom;
        let mut node = self.root[((y >> shift) as usize) << side_bits | (x >> shift) as usize];
        let mut depth = self.root_zoom;
        while node != NO_TILE && node >= self.block_base {
            depth += 2;
            let base = usize::from(node - self.block_base) * BLOCK_CELLS;
            node = self.blocks[base + child_slot(x, y, self.leaf_zoom - depth)];
        }
        (node != NO_TILE).then_some(node)
    }

    /// All tile keys carrying any of the given timezone indices, sorted
    /// ascending — the zoom lives in the key's high bits, so coarser tiles
    /// come first.
    #[cfg(feature = "export-geojson")]
    pub(crate) fn tile_keys_for(&self, indices: &[u16]) -> Vec<u64> {
        self.tiles
            .iter()
            .filter(|(_, entry)| entry.indices().iter().any(|i| indices.contains(i)))
            .map(|(key, _)| *key)
            .collect()
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
        grouped
    }
}

/// Merges identical blocks and narrows the nodes to [`Node`].
///
/// The grid is read-only once built, so blocks with the same content can be
/// shared: on the bundled data about half of them repeat. A block is always
/// created after its parent, so walking blocks from the last one back meets
/// every child before its parent, and a parent's content is compared with its
/// child pointers already merged. Blocks no node reaches (their area was
/// overwritten by a coarser tile) are dropped.
fn compact(
    root: &[BuildNode],
    blocks: &[BuildNode],
    block_base: BuildNode,
) -> Result<(Nodes, Nodes), Error> {
    let is_block = |v: BuildNode| v != BUILD_NO_TILE && v >= block_base;
    let count = blocks.len() / BLOCK_CELLS;
    let mut reachable = vec![false; count];
    for &v in root.iter().filter(|&&v| is_block(v)) {
        reachable[(v - block_base) as usize] = true;
    }
    // Children have larger indices than their parents.
    for b in 0..count {
        if reachable[b] {
            for &v in blocks[b * BLOCK_CELLS..][..BLOCK_CELLS]
                .iter()
                .filter(|&&v| is_block(v))
            {
                reachable[(v - block_base) as usize] = true;
            }
        }
    }

    let narrow = |v: BuildNode, remap: &[BuildNode]| -> Result<Node, Error> {
        let v = if is_block(v) {
            block_base + remap[(v - block_base) as usize]
        } else if v == BUILD_NO_TILE {
            return Ok(NO_TILE);
        } else {
            v
        };
        Node::try_from(v)
            .ok()
            .filter(|&v| v < NO_TILE)
            .ok_or(Error::Malformed("FUZZY grid exceeds its node range"))
    };
    let mut remap: Vec<BuildNode> = vec![0; count];
    let mut seen: FxHashMap<[Node; BLOCK_CELLS], BuildNode> =
        FxHashMap::with_capacity_and_hasher(count, FxBuildHasher);
    // Merged blocks in the order first met (children first); reversed below
    // so parents precede children in memory, as they did when built.
    let mut merged: Vec<[Node; BLOCK_CELLS]> = Vec::new();
    for b in (0..count).rev().filter(|&b| reachable[b]) {
        let mut cells = [NO_TILE; BLOCK_CELLS];
        for (cell, &v) in cells
            .iter_mut()
            .zip(&blocks[b * BLOCK_CELLS..][..BLOCK_CELLS])
        {
            *cell = narrow(v, &remap)?;
        }
        remap[b] = *seen.entry(cells).or_insert_with(|| {
            merged.push(cells);
            (merged.len() - 1) as BuildNode
        });
    }
    // Reverse the merged order: id i becomes len - 1 - i.
    let last = merged.len().saturating_sub(1) as BuildNode;
    let flip = |v: Node| -> Node {
        if v != NO_TILE && BuildNode::from(v) >= block_base {
            // Fits: both ids are below the merged count already checked.
            #[allow(clippy::cast_possible_truncation)]
            let id = (BuildNode::from(v) - block_base) as Node;
            #[allow(clippy::cast_possible_truncation)]
            {
                (block_base as Node) + (last as Node - id)
            }
        } else {
            v
        }
    };
    let blocks: Nodes = merged
        .iter()
        .rev()
        .flat_map(|cells| cells.iter().map(|&v| flip(v)))
        .collect();
    let root: Nodes = root
        .iter()
        .map(|&v| narrow(v, &remap).map(flip))
        .collect::<Result<_, _>>()?;
    Ok((root, blocks))
}

/// Position inside a block of the child holding cell (x, y), where the cells
/// are `below` zooms finer than the block's children.
#[inline]
fn child_slot(x: u32, y: u32, below: u8) -> usize {
    let mask = BLOCK_SIDE - 1;
    (((y >> below) & mask) * BLOCK_SIDE + ((x >> below) & mask)) as usize
}

#[cfg(all(test, any(feature = "bundled", feature = "full")))]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// The reference lookup: a hash probe per zoom from `agg_zoom` to
    /// `idx_zoom`, coarsest first — the pre-grid implementation.
    struct ProbeEveryZoom {
        idx_zoom: u8,
        agg_zoom: u8,
        tiles: HashMap<u64, u16>,
    }

    impl ProbeEveryZoom {
        fn new(reader: &Reader<'_>) -> Self {
            let (idx_zoom, agg_zoom) = reader.fuzzy_zooms().unwrap();
            let tiles = reader
                .fuzzy_entries()
                .unwrap()
                .into_iter()
                .map(|(key, indices)| (key, indices[0]))
                .collect();
            Self {
                idx_zoom,
                agg_zoom,
                tiles,
            }
        }

        fn get(&self, lng: f64, lat: f64) -> Option<u16> {
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
                .copied()
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
    fn grid_matches_probing_every_zoom() {
        check_grid(crate::load_lite_tzb());
    }

    #[cfg(feature = "full")]
    #[test]
    fn grid_matches_probing_every_zoom_full() {
        check_grid(tzf_dist_git::load_full_tzb());
    }

    /// Optional extra datasets (e.g. preindex variants kept to finer zooms),
    /// colon-separated paths in `TZF_FUZZY_GRID_EXTRA`.
    #[test]
    fn grid_matches_probing_every_zoom_extra() {
        let Ok(paths) = std::env::var("TZF_FUZZY_GRID_EXTRA") else {
            return;
        };
        for path in paths.split(':').filter(|p| !p.is_empty()) {
            check_grid(&std::fs::read(path).unwrap());
        }
    }

    fn check_grid(data: &[u8]) {
        let reader = Reader::open(std::borrow::Cow::Borrowed(data)).unwrap();
        let fuzzy = FuzzyIndex::from_reader(&reader).unwrap().unwrap();
        let reference = ProbeEveryZoom::new(&reader);

        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let mut points: Vec<(f64, f64)> = (0..300_000)
            .map(|_| (rng.next() * 360.0 - 180.0, rng.next() * 180.0 - 90.0))
            .collect();
        // polar caps, where the Mercator rows are clamped
        points.extend((0..50_000).map(|_| (rng.next() * 360.0 - 180.0, 80.0 + rng.next() * 10.0)));
        points.extend((0..50_000).map(|_| (rng.next() * 360.0 - 180.0, -90.0 + rng.next() * 10.0)));
        // tile corners and centres at every zoom the grid stores, where a
        // block or root boundary sits
        for z in 0..=fuzzy.leaf_zoom {
            let n = f64::from(1u32 << z);
            for _ in 0..2_000 {
                let (tx, ty) = ((rng.next() * n).floor(), (rng.next() * n).floor());
                for (fx, fy) in [(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)] {
                    let lng = (tx + fx) / n * 360.0 - 180.0;
                    let lat = (std::f64::consts::PI * (1.0 - 2.0 * (ty + fy) / n))
                        .sinh()
                        .atan()
                        .to_degrees();
                    points.push((lng, lat));
                }
            }
        }
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

        points.extend([
            (f64::NAN, 0.0),
            (0.0, f64::NAN),
            (180.000_001, 0.0),
            (0.0, -90.000_001),
        ]);

        let mut hits = 0;
        for &(lng, lat) in &points {
            let got = fuzzy.get(lng, lat);
            assert_eq!(got, reference.get(lng, lat), "at ({lng}, {lat})");
            hits += usize::from(got.is_some());
        }
        assert!(hits > 0, "the sample never reached the fuzzy index");
    }
}
