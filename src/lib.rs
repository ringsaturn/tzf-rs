#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

use geometry_rs::{
    CoordStorage, I32Point, I32Polygon, I32RaycastMode, Point, Polygon, PolygonBuildOptions,
};
#[cfg(feature = "export-geojson")]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::vec;
#[cfg(all(feature = "bundled", feature = "full"))]
compile_error!(
    "features `bundled` and `full` are mutually exclusive; \
     add `default-features = false` when enabling `full`"
);

#[cfg(feature = "bundled")]
use tzf_dist::{load_preindex, load_topology_compress_topo};
#[cfg(feature = "full")]
use tzf_dist_git::{load_compress_topo, load_preindex, load_topology_compress_topo};
pub mod pbgen;

struct Item<T: CoordStorage> {
    polys: Vec<Polygon<T>>,
    name: String,
}

impl<T: CoordStorage> Item<T> {
    fn contains_point(&self, p: &Point) -> bool {
        for poly in &self.polys {
            if poly.contains_point(*p) {
                return true;
            }
        }
        false
    }
}

/// Monomorphized finder internals. `T` is the polygon coordinate storage:
/// `i32` (1e5-scaled) for compressed topo data, `f64` for user-supplied
/// protobuf data.
struct FinderCore<T: CoordStorage> {
    all: Vec<Item<T>>,
    data_version: String,
    // grid maps (floor(lng), floor(lat)) → candidate item indices.
    // Populated automatically when loading CompressedTopoTimezones that
    // contains an embedded GridIndex.
    grid: Option<HashMap<(i16, i16), Vec<u32>>>,
}

enum FinderKind {
    Float(FinderCore<f64>),
    Scaled(FinderCore<i32>),
}

/// Dispatch once at the top of each query; everything below the dispatch is
/// monomorphized over the storage type, avoiding a per-polygon enum match.
macro_rules! with_core {
    ($finder:expr, $core:ident => $body:expr) => {
        match &$finder.inner {
            FinderKind::Float($core) => $body,
            FinderKind::Scaled($core) => $body,
        }
    };
}

/// Finder works anywhere.
///
/// Finder use a fine tuned Ray casting algorithm implement [geometry-rs]
/// which is Rust port of [geometry] by [Josh Baker].
///
/// [geometry-rs]: https://github.com/ringsaturn/geometry-rs
/// [geometry]: https://github.com/tidwall/geometry
/// [Josh Baker]: https://github.com/tidwall
pub struct Finder {
    inner: FinderKind,
}

const DEFAULT_RTREE_MIN_SEGMENTS: usize = 64;

/// Finder build options for polygon acceleration indexes.
///
/// Compressed topo data (the tzf-dist default) always stores polygons as
/// 1e5-scaled integer coordinates; the options only choose the acceleration
/// index and the raycast flavor. The indexes operate directly in the scaled
/// integer storage space, so [`FinderOptions::YStripes`] keeps the full
/// memory savings of integer storage.
///
/// Default:
/// - [`FinderOptions::NoIndex`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum FinderOptions {
    /// Disable polygon acceleration indexes.
    ///
    /// For compressed topo data this is equivalent to
    /// [`FinderOptions::NoIndexFloatRaycast`].
    #[default]
    NoIndex,
    /// Use Y stripes index (recommended).
    YStripes,
    /// Disable polygon acceleration indexes; segment endpoints are converted
    /// to `f64` in registers during raycasting.
    NoIndexFloatRaycast,
    /// Disable polygon acceleration indexes and use an integer cross-product
    /// raycast, which snaps the query point to the 1e-5 grid (a semantic
    /// difference near polygon edges). Opt-in.
    NoIndexIntegerRaycast,
}

impl FinderOptions {
    /// Disable polygon acceleration indexes.
    #[must_use]
    pub fn no_index() -> Self {
        Self::NoIndex
    }

    /// Use Y stripes index.
    #[must_use]
    pub fn y_stripes() -> Self {
        Self::YStripes
    }

    #[must_use]
    pub fn no_index_float_raycast() -> Self {
        Self::NoIndexFloatRaycast
    }

    #[must_use]
    pub fn no_index_integer_raycast() -> Self {
        Self::NoIndexIntegerRaycast
    }

    fn to_polygon_build_options(self) -> PolygonBuildOptions {
        match self {
            Self::YStripes => PolygonBuildOptions {
                enable_rtree: false,
                enable_compressed_quad: false,
                enable_y_stripes: true,
                rtree_min_segments: DEFAULT_RTREE_MIN_SEGMENTS,
            },
            Self::NoIndex | Self::NoIndexFloatRaycast | Self::NoIndexIntegerRaycast => {
                PolygonBuildOptions {
                    enable_rtree: false,
                    enable_compressed_quad: false,
                    enable_y_stripes: false,
                    rtree_min_segments: DEFAULT_RTREE_MIN_SEGMENTS,
                }
            }
        }
    }

    fn i32_raycast_mode(self) -> I32RaycastMode {
        match self {
            Self::NoIndexIntegerRaycast => I32RaycastMode::Integer,
            Self::NoIndex | Self::NoIndexFloatRaycast | Self::YStripes => I32RaycastMode::Float,
        }
    }
}

/// Decode a Google Polyline encoded byte slice into a list of Points.
///
/// The go-polyline library encodes coordinates as [lng, lat] pairs with 1e5 precision.
#[allow(clippy::cast_possible_truncation)]
fn decode_polyline(encoded: &[u8]) -> Vec<I32Point> {
    let mut points = Vec::new();
    let mut index = 0;
    let mut lng: i64 = 0;
    let mut lat: i64 = 0;

    while index < encoded.len() {
        let (dlng, next) = polyline_decode_value(encoded, index);
        index = next;
        let (dlat, next) = polyline_decode_value(encoded, index);
        index = next;
        lng += dlng;
        lat += dlat;
        points.push(I32Point {
            x: i32::try_from(lng).expect("polyline longitude exceeds i32"),
            y: i32::try_from(lat).expect("polyline latitude exceeds i32"),
        });
    }
    points
}

fn polyline_decode_value(encoded: &[u8], start: usize) -> (i64, usize) {
    let mut result: i64 = 0;
    let mut shift = 0;
    let mut index = start;

    loop {
        let byte = (encoded[index] as i64) - 63;
        index += 1;
        result |= (byte & 0x1F) << shift;
        shift += 5;
        if byte < 0x20 {
            break;
        }
    }

    let value = if result & 1 != 0 {
        !(result >> 1)
    } else {
        result >> 1
    };
    (value, index)
}

fn expand_compressed_ring(
    segs: &[pbgen::CompressedRingSegment],
    edges: &[Vec<I32Point>],
) -> Vec<I32Point> {
    let mut pts = Vec::new();
    for seg in segs {
        match &seg.content {
            Some(pbgen::compressed_ring_segment::Content::Inline(inline)) => {
                pts.extend(decode_polyline(&inline.points));
            }
            Some(pbgen::compressed_ring_segment::Content::EdgeForward(idx)) => {
                pts.extend_from_slice(&edges[*idx as usize]);
            }
            Some(pbgen::compressed_ring_segment::Content::EdgeReversed(idx)) => {
                pts.extend(edges[*idx as usize].iter().rev().copied());
            }
            None => {}
        }
    }
    pts
}

impl<T: CoordStorage> FinderCore<T> {
    fn get_tz_name(&self, lng: f64, lat: f64) -> &str {
        if let Some(ref grid) = self.grid {
            let key = (lng.floor() as i16, lat.floor() as i16);
            let indices = match grid.get(&key) {
                Some(v) => v,
                None => return "",
            };
            // Single-candidate short-circuit: skip PIP when there is only one
            // candidate and we are away from antimeridian / pole edges.
            if indices.len() == 1 && (-179.0..179.0).contains(&lng) && (-89.0..89.0).contains(&lat)
            {
                return &self.all[indices[0] as usize].name;
            }
            let p = geometry_rs::Point { x: lng, y: lat };
            for &idx in indices {
                if self.all[idx as usize].contains_point(&p) {
                    return &self.all[idx as usize].name;
                }
            }
            return "";
        }
        let p = geometry_rs::Point { x: lng, y: lat };
        for item in &self.all {
            if item.contains_point(&p) {
                return &item.name;
            }
        }
        ""
    }

    fn get_tz_names(&self, lng: f64, lat: f64) -> Vec<&str> {
        let mut ret: Vec<&str> = vec![];
        if let Some(ref grid) = self.grid {
            let key = (lng.floor() as i16, lat.floor() as i16);
            if let Some(indices) = grid.get(&key) {
                let p = geometry_rs::Point { x: lng, y: lat };
                for &idx in indices {
                    if self.all[idx as usize].contains_point(&p) {
                        ret.push(&self.all[idx as usize].name);
                    }
                }
            }
            return ret;
        }
        let p = geometry_rs::Point { x: lng, y: lat };
        for item in &self.all {
            if item.contains_point(&p) {
                ret.push(&item.name);
            }
        }
        ret
    }

    fn timezonenames(&self) -> Vec<&str> {
        let mut ret: Vec<&str> = vec![];
        for item in &self.all {
            ret.push(&item.name);
        }
        ret
    }
}

impl Finder {
    fn from_pb_with_polygon_options(tzs: pbgen::Timezones, options: PolygonBuildOptions) -> Self {
        let mut all: Vec<Item<f64>> = vec![];
        for tz in &tzs.timezones {
            let mut polys: Vec<Polygon> = vec![];

            for pbpoly in &tz.polygons {
                let mut exterior: Vec<Point> = vec![];
                for pbpoint in &pbpoly.points {
                    exterior.push(Point {
                        x: f64::from(pbpoint.lng),
                        y: f64::from(pbpoint.lat),
                    });
                }

                let mut interior: Vec<Vec<Point>> = vec![];

                for holepoly in &pbpoly.holes {
                    let mut holeextr: Vec<Point> = vec![];
                    for holepoint in &holepoly.points {
                        holeextr.push(Point {
                            x: f64::from(holepoint.lng),
                            y: f64::from(holepoint.lat),
                        });
                    }
                    interior.push(holeextr);
                }

                polys.push(geometry_rs::Polygon::new(exterior, interior, Some(options)));
            }

            all.push(Item {
                name: tz.name.to_string(),
                polys,
            });
        }
        Self {
            inner: FinderKind::Float(FinderCore {
                all,
                data_version: tzs.version,
                grid: None,
            }),
        }
    }

    fn from_compressed_topo_with_polygon_options(
        tzs: pbgen::CompressedTopoTimezones,
        options: PolygonBuildOptions,
        raycast_mode: I32RaycastMode,
    ) -> Self {
        let mut edges: Vec<Vec<I32Point>> = vec![Vec::new(); tzs.shared_edges.len()];
        for edge in &tzs.shared_edges {
            edges[edge.id as usize] = decode_polyline(&edge.points);
        }

        let grid = tzs.grid_index.map(|gi| {
            let mut m = HashMap::with_capacity(gi.cells.len());
            for cell in gi.cells {
                m.insert((cell.lng as i16, cell.lat as i16), cell.tz_indices);
            }
            m
        });

        let mut all: Vec<Item<i32>> = vec![];
        for tz in &tzs.timezones {
            let mut polys: Vec<I32Polygon> = vec![];
            for poly in &tz.polygons {
                let exterior = expand_compressed_ring(&poly.exterior, &edges);
                let interior: Vec<Vec<I32Point>> = poly
                    .holes
                    .iter()
                    .map(|hole| expand_compressed_ring(&hole.exterior, &edges))
                    .collect();
                // The acceleration indexes operate directly in the 1e5-scaled
                // integer storage space, so enabling them no longer requires
                // falling back to float storage.
                polys.push(I32Polygon::new_with_options(
                    exterior,
                    interior,
                    1e5,
                    raycast_mode,
                    Some(options),
                ));
            }
            all.push(Item {
                name: tz.name.clone(),
                polys,
            });
        }
        Self {
            inner: FinderKind::Scaled(FinderCore {
                all,
                data_version: tzs.version,
                grid,
            }),
        }
    }

    /// Create a Finder from `CompressedTopoTimezones` protobuf data.
    ///
    /// This is the preferred constructor when using tzf-dist data.
    #[must_use]
    pub fn from_compressed_topo(tzs: pbgen::CompressedTopoTimezones) -> Self {
        Self::from_compressed_topo_with_options(tzs, FinderOptions::default())
    }

    /// Create a Finder from `CompressedTopoTimezones` with explicit polygon build options.
    #[must_use]
    pub fn from_compressed_topo_with_options(
        tzs: pbgen::CompressedTopoTimezones,
        options: FinderOptions,
    ) -> Self {
        Self::from_compressed_topo_with_polygon_options(
            tzs,
            options.to_polygon_build_options(),
            options.i32_raycast_mode(),
        )
    }

    /// `from_pb` is used when you can use your own timezone data, as long as
    /// it's compatible with Proto's desc.
    ///
    /// # Arguments
    ///
    /// * `tzs` - Timezones data.
    ///
    /// # Returns
    ///
    /// * `Finder` - A Finder instance.
    #[must_use]
    pub fn from_pb(tzs: pbgen::Timezones) -> Self {
        Self::from_pb_with_options(tzs, FinderOptions::default())
    }

    /// Create a finder from protobuf data with explicit polygon build options.
    #[must_use]
    pub fn from_pb_with_options(tzs: pbgen::Timezones, options: FinderOptions) -> Self {
        Self::from_pb_with_polygon_options(tzs, options.to_polygon_build_options())
    }

    /// Example:
    ///
    /// ```rust
    /// use tzf_rs::Finder;
    ///
    /// let finder = Finder::new();
    /// assert_eq!("Asia/Shanghai", finder.get_tz_name(116.3883, 39.9289));
    /// ```
    #[must_use]
    pub fn get_tz_name(&self, lng: f64, lat: f64) -> &str {
        with_core!(self, core => core.get_tz_name(lng, lat))
    }

    /// ```rust
    /// use tzf_rs::Finder;
    /// let finder = Finder::new();
    /// println!("{:?}", finder.get_tz_names(116.3883, 39.9289));
    /// ```
    #[must_use]
    pub fn get_tz_names(&self, lng: f64, lat: f64) -> Vec<&str> {
        with_core!(self, core => core.get_tz_names(lng, lat))
    }

    /// Example:
    ///
    /// ```rust
    /// use tzf_rs::Finder;
    ///
    /// let finder = Finder::new();
    /// println!("{:?}", finder.timezonenames());
    /// ```
    #[must_use]
    pub fn timezonenames(&self) -> Vec<&str> {
        with_core!(self, core => core.timezonenames())
    }

    /// Example:
    ///
    /// ```rust
    /// use tzf_rs::Finder;
    ///
    /// let finder = Finder::new();
    /// println!("{:?}", finder.data_version());
    /// ```
    #[must_use]
    pub fn data_version(&self) -> &str {
        with_core!(self, core => &core.data_version)
    }

    /// Creates a new, empty `Finder`.
    ///
    /// Example:
    ///
    /// ```rust
    /// use tzf_rs::Finder;
    ///
    /// let finder = Finder::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert the Finder's data to GeoJSON format.
    ///
    /// Returns a `BoundaryFile` (FeatureCollection) containing all timezone polygons.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tzf_rs::Finder;
    ///
    /// let finder = Finder::new();
    /// let geojson = finder.to_geojson();
    /// let json_string = geojson.to_string();
    /// ```
    #[must_use]
    #[cfg(feature = "export-geojson")]
    pub fn to_geojson(&self) -> BoundaryFile {
        with_core!(self, core => core.to_geojson())
    }

    /// Convert a specific timezone to GeoJSON format.
    ///
    /// Returns `Some(BoundaryFile)` containing a FeatureCollection with all features
    /// for the timezone if found, `None` otherwise. The returned FeatureCollection
    /// may contain multiple features if the timezone has multiple geographic boundaries.
    ///
    /// # Arguments
    ///
    /// * `timezone_name` - The timezone name to export (e.g., "Asia/Tokyo")
    ///
    /// # Example
    ///
    /// ```rust
    /// use tzf_rs::Finder;
    ///
    /// let finder = Finder::new();
    /// if let Some(collection) = finder.get_tz_geojson("Asia/Tokyo") {
    ///     let json_string = collection.to_string();
    ///     println!("Found {} feature(s)", collection.features.len());
    ///     if let Some(first_feature) = collection.features.first() {
    ///         println!("Timezone ID: {}", first_feature.properties.tzid);
    ///     }
    /// }
    /// ```
    #[must_use]
    #[cfg(feature = "export-geojson")]
    pub fn get_tz_geojson(&self, timezone_name: &str) -> Option<BoundaryFile> {
        with_core!(self, core => core.get_tz_geojson(timezone_name))
    }
}

#[cfg(feature = "export-geojson")]
impl<T: CoordStorage> FinderCore<T> {
    /// Helper method to convert an Item to a FeatureItem.
    fn item_to_feature(&self, item: &Item<T>) -> FeatureItem {
        // Convert internal Item to pbgen::Timezone format
        let mut pbpolys = Vec::new();
        for poly in &item.polys {
            // Storage space → degrees; `scale` is 1.0 for float storage.
            let scale = poly.scale();
            let mut pbpoly = pbgen::Polygon {
                points: Vec::new(),
                holes: Vec::new(),
            };

            pbpoly
                .points
                .extend(poly.exterior().iter().map(|point| pbgen::Point {
                    lng: (point.x.to_f64() / scale) as f32,
                    lat: (point.y.to_f64() / scale) as f32,
                }));
            for hole in poly.holes() {
                pbpoly.holes.push(pbgen::Polygon {
                    points: hole
                        .iter()
                        .map(|point| pbgen::Point {
                            lng: (point.x.to_f64() / scale) as f32,
                            lat: (point.y.to_f64() / scale) as f32,
                        })
                        .collect(),
                    holes: Vec::new(),
                });
            }

            pbpolys.push(pbpoly);
        }

        let pbtz = pbgen::Timezone {
            polygons: pbpolys,
            name: item.name.clone(),
        };

        revert_item(&pbtz)
    }

    fn to_geojson(&self) -> BoundaryFile {
        let mut output = BoundaryFile {
            collection_type: "FeatureCollection".to_string(),
            features: Vec::new(),
        };

        for item in &self.all {
            output.features.push(self.item_to_feature(item));
        }

        output
    }

    fn get_tz_geojson(&self, timezone_name: &str) -> Option<BoundaryFile> {
        let mut output = BoundaryFile {
            collection_type: "FeatureCollection".to_string(),
            features: Vec::new(),
        };
        for item in &self.all {
            if item.name == timezone_name {
                output.features.push(self.item_to_feature(item));
            }
        }

        if output.features.is_empty() {
            None
        } else {
            Some(output)
        }
    }
}

/// Creates a new, empty `Finder`.
///
/// Example:
///
/// ```rust
/// use tzf_rs::Finder;
///
/// let finder = Finder::default();
/// ```
impl Default for Finder {
    fn default() -> Self {
        let file_bytes = load_topology_compress_topo();
        Self::from_compressed_topo(
            pbgen::CompressedTopoTimezones::try_from(file_bytes).unwrap_or_default(),
        )
    }
}

/// deg2num is used to convert longitude, latitude to [Slippy map tilenames]
/// under specific zoom level.
///
/// [Slippy map tilenames]: https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames
///
/// Example:
///
/// ```rust
/// use tzf_rs::deg2num;
/// let ret = deg2num(116.3883, 39.9289, 7);
/// assert_eq!((105, 48), ret);
/// ```
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::similar_names
)]
pub fn deg2num(lng: f64, lat: f64, zoom: i64) -> (i64, i64) {
    let n = (1i64 << zoom) as f64;
    let lat_rad = lat.to_radians();
    let xtile = (lng / 360.0 + 0.5) * n;
    let ytile = (1.0 - lat_rad.tan().asinh() / PI) / 2.0 * n;

    // Possible precision loss here
    (xtile as i64, ytile as i64)
}

/// GeoJSON type definitions for conversion
#[cfg(feature = "export-geojson")]
pub type PolygonCoordinates = Vec<Vec<[f64; 2]>>;
#[cfg(feature = "export-geojson")]
pub type MultiPolygonCoordinates = Vec<PolygonCoordinates>;

#[cfg(feature = "export-geojson")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeometryDefine {
    #[serde(rename = "type")]
    pub geometry_type: String,
    pub coordinates: MultiPolygonCoordinates,
}

#[cfg(feature = "export-geojson")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertiesDefine {
    pub tzid: String,
}

#[cfg(feature = "export-geojson")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureItem {
    #[serde(rename = "type")]
    pub feature_type: String,
    pub properties: PropertiesDefine,
    pub geometry: GeometryDefine,
}

#[cfg(feature = "export-geojson")]
impl FeatureItem {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn to_string_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

#[cfg(feature = "export-geojson")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryFile {
    #[serde(rename = "type")]
    pub collection_type: String,
    pub features: Vec<FeatureItem>,
}

#[cfg(feature = "export-geojson")]
impl BoundaryFile {
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn to_string_pretty(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// Convert protobuf Polygon array to GeoJSON MultiPolygon coordinates
#[cfg(feature = "export-geojson")]
fn from_pb_polygon_to_geo_multipolygon(pbpoly: &[pbgen::Polygon]) -> MultiPolygonCoordinates {
    let mut res = MultiPolygonCoordinates::new();
    for poly in pbpoly {
        let mut new_geo_poly = PolygonCoordinates::new();

        // Main polygon (exterior ring)
        let mut mainpoly = Vec::new();
        for point in &poly.points {
            mainpoly.push([f64::from(point.lng), f64::from(point.lat)]);
        }
        new_geo_poly.push(mainpoly);

        // Holes (interior rings)
        for holepoly in &poly.holes {
            let mut holepoly_coords = Vec::new();
            for point in &holepoly.points {
                holepoly_coords.push([f64::from(point.lng), f64::from(point.lat)]);
            }
            new_geo_poly.push(holepoly_coords);
        }
        res.push(new_geo_poly);
    }
    res
}

/// Convert a protobuf Timezone to a GeoJSON FeatureItem
#[cfg(feature = "export-geojson")]
fn revert_item(input: &pbgen::Timezone) -> FeatureItem {
    FeatureItem {
        feature_type: "Feature".to_string(),
        properties: PropertiesDefine {
            tzid: input.name.clone(),
        },
        geometry: GeometryDefine {
            geometry_type: "MultiPolygon".to_string(),
            coordinates: from_pb_polygon_to_geo_multipolygon(&input.polygons),
        },
    }
}

/// Convert protobuf Timezones to GeoJSON BoundaryFile (FeatureCollection)
#[cfg(feature = "export-geojson")]
pub fn revert_timezones(input: &pbgen::Timezones) -> BoundaryFile {
    let mut output = BoundaryFile {
        collection_type: "FeatureCollection".to_string(),
        features: Vec::new(),
    };
    for timezone in &input.timezones {
        let item = revert_item(timezone);
        output.features.push(item);
    }
    output
}

// Packs (x, y, z) into a single u64 tile key, mirroring the Go
// implementation's TileID layout:
// bits 56-63 = zoom (0-255), bits 28-55 = x (up to 2^28), bits 0-27 = y (up to 2^28).
// This covers all OSM zoom levels (0-28) without collision. Out-of-range
// lookup coordinates are masked; the masked values exceed any real tile
// index so they simply never match.
const TILE_COORD_BITS: u32 = 28;
const TILE_COORD_MASK: u64 = (1 << TILE_COORD_BITS) - 1;

#[inline]
#[allow(clippy::cast_sign_loss)]
fn pack_tile_key(x: i64, y: i64, z: i64) -> u64 {
    ((z as u64) << (2 * TILE_COORD_BITS))
        | ((x as u64 & TILE_COORD_MASK) << TILE_COORD_BITS)
        | (y as u64 & TILE_COORD_MASK)
}

#[cfg(feature = "export-geojson")]
#[allow(clippy::cast_possible_wrap)]
fn unpack_tile_key(key: u64) -> (i64, i64, i64) {
    let x = ((key >> TILE_COORD_BITS) & TILE_COORD_MASK) as i64;
    let y = (key & TILE_COORD_MASK) as i64;
    let z = (key >> (2 * TILE_COORD_BITS)) as i64;
    (x, y, z)
}

/// Most tiles belong to exactly one timezone, so store that index inline and
/// only heap-allocate for boundary tiles that straddle multiple timezones.
enum TileEntry {
    One(u16),
    Many(Box<[u16]>),
}

impl TileEntry {
    fn indices(&self) -> &[u16] {
        match self {
            Self::One(idx) => std::slice::from_ref(idx),
            Self::Many(idxs) => idxs,
        }
    }
}

/// `FuzzyFinder` blazing fast for most places on earth, use a preindex data.
/// Not work for places around borders.
///
/// `FuzzyFinder` store all preindex's tiles data in a `HashMap`,
/// It iterate all zoom levels for input's longitude and latitude to build
/// map key to to check if in map.
///
/// It's is very fast and use about 400ns to check if has preindex.
/// It work for most places on earth and here is a quick loop of preindex data:
/// ![](https://user-images.githubusercontent.com/13536789/200174943-7d40661e-bda5-4b79-a867-ec637e245a49.png)
pub struct FuzzyFinder {
    min_zoom: i64,
    max_zoom: i64,
    // Sorted timezone name table; tiles reference names by index, so index
    // order matches lexical order.
    names: Vec<String>,
    all: HashMap<u64, TileEntry>, // K: packed <x,y,z>
    data_version: String,
}

impl Default for FuzzyFinder {
    /// Creates a new, empty `FuzzyFinder`.
    ///
    /// ```rust
    /// use tzf_rs::FuzzyFinder;
    ///
    /// let finder = FuzzyFinder::default();
    /// ```
    fn default() -> Self {
        let file_bytes = load_preindex();
        Self::from_pb(pbgen::PreindexTimezones::try_from(file_bytes.to_vec()).unwrap_or_default())
    }
}

impl FuzzyFinder {
    /// # Panics
    ///
    /// Panics if the input contains more than `u16::MAX` distinct timezone names.
    #[must_use]
    pub fn from_pb(tzs: pbgen::PreindexTimezones) -> Self {
        // First pass: build a sorted name table so indices compare in the
        // same order as the names themselves.
        let mut names: Vec<String> = tzs.keys.iter().map(|item| item.name.clone()).collect();
        names.sort();
        names.dedup();
        let name_idx: HashMap<&str, u16> = names
            .iter()
            .enumerate()
            .map(|(i, name)| {
                (
                    name.as_str(),
                    u16::try_from(i).expect("more than u16::MAX timezone names"),
                )
            })
            .collect();

        // Second pass: populate tiles with name indices.
        let mut all: HashMap<u64, TileEntry> = HashMap::new();
        for item in &tzs.keys {
            let idx = name_idx[item.name.as_str()];
            let key = pack_tile_key(i64::from(item.x), i64::from(item.y), i64::from(item.z));
            match all.entry(key) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(TileEntry::One(idx));
                }
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    let mut idxs = entry.get().indices().to_vec();
                    idxs.push(idx);
                    idxs.sort_unstable();
                    *entry.get_mut() = TileEntry::Many(idxs.into_boxed_slice());
                }
            }
        }

        Self {
            min_zoom: i64::from(tzs.agg_zoom),
            max_zoom: i64::from(tzs.idx_zoom),
            names,
            all,
            data_version: tzs.version,
        }
    }

    /// Retrieves the time zone name for the given longitude and latitude.
    ///
    /// # Arguments
    ///
    /// * `lng` - Longitude
    /// * `lat` - Latitude
    ///
    /// # Example:
    ///
    /// ```rust
    /// use tzf_rs::FuzzyFinder;
    ///
    /// let finder = FuzzyFinder::new();
    /// assert_eq!("Asia/Shanghai", finder.get_tz_name(116.3883, 39.9289));
    /// ```
    ///
    /// # Panics
    ///
    /// - Panics if `lng` or `lat` is out of range.
    /// - Panics if `lng` or `lat` is not a number.
    #[must_use]
    pub fn get_tz_name(&self, lng: f64, lat: f64) -> &str {
        if self.max_zoom <= self.min_zoom {
            return "";
        }
        // Compute tile coords once at the highest zoom, then right-shift for coarser levels.
        let top_zoom = self.max_zoom - 1;
        let (high_x, high_y) = deg2num(lng, lat, top_zoom);
        for zoom in self.min_zoom..self.max_zoom {
            let shift = (top_zoom - zoom) as u32;
            if let Some(&idx) = self
                .all
                .get(&pack_tile_key(high_x >> shift, high_y >> shift, zoom))
                .and_then(|entry| entry.indices().first())
            {
                return &self.names[usize::from(idx)];
            }
        }
        ""
    }

    pub fn get_tz_names(&self, lng: f64, lat: f64) -> Vec<&str> {
        let mut names: Vec<&str> = vec![];
        if self.max_zoom <= self.min_zoom {
            return names;
        }
        let top_zoom = self.max_zoom - 1;
        let (high_x, high_y) = deg2num(lng, lat, top_zoom);
        for zoom in self.min_zoom..self.max_zoom {
            let shift = (top_zoom - zoom) as u32;
            if let Some(entry) =
                self.all
                    .get(&pack_tile_key(high_x >> shift, high_y >> shift, zoom))
            {
                for &idx in entry.indices() {
                    names.push(self.names[usize::from(idx)].as_str());
                }
            }
        }
        names
    }

    /// Gets the version of the data used by this `FuzzyFinder`.
    ///
    /// # Returns
    ///
    /// The version of the data used by this `FuzzyFinder` as a `&str`.
    ///
    /// # Example:
    ///
    /// ```rust
    /// use tzf_rs::FuzzyFinder;
    ///
    /// let finder = FuzzyFinder::new();
    /// println!("{:?}", finder.data_version());
    /// ```
    #[must_use]
    pub fn data_version(&self) -> &str {
        &self.data_version
    }

    /// Creates a new, empty `FuzzyFinder`.
    ///
    /// ```rust
    /// use tzf_rs::FuzzyFinder;
    ///
    /// let finder = FuzzyFinder::default();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert the FuzzyFinder's preindex data to GeoJSON format.
    ///
    /// This method generates polygons for each tile in the preindex,
    /// representing the geographic bounds of each tile.
    ///
    /// Returns a `BoundaryFile` (FeatureCollection) containing all timezone tile polygons.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tzf_rs::FuzzyFinder;
    ///
    /// let finder = FuzzyFinder::new();
    /// let geojson = finder.to_geojson();
    /// let json_string = geojson.to_string();
    /// ```
    #[must_use]
    #[cfg(feature = "export-geojson")]
    pub fn to_geojson(&self) -> BoundaryFile {
        let mut name_to_keys: HashMap<u16, Vec<(i64, i64, i64)>> = HashMap::new();

        // Group tiles by timezone name index
        for (key, entry) in &self.all {
            for &idx in entry.indices() {
                name_to_keys
                    .entry(idx)
                    .or_default()
                    .push(unpack_tile_key(*key));
            }
        }

        let mut features = Vec::new();

        for (idx, keys) in name_to_keys {
            let mut multi_polygon_coords = MultiPolygonCoordinates::new();

            for (x, y, z) in keys {
                // Convert tile coordinates to lat/lng bounds
                let tile_poly = tile_to_polygon(x, y, z);
                multi_polygon_coords.push(vec![tile_poly]);
            }

            let feature = FeatureItem {
                feature_type: "Feature".to_string(),
                properties: PropertiesDefine {
                    tzid: self.names[usize::from(idx)].clone(),
                },
                geometry: GeometryDefine {
                    geometry_type: "MultiPolygon".to_string(),
                    coordinates: multi_polygon_coords,
                },
            };

            features.push(feature);
        }

        BoundaryFile {
            collection_type: "FeatureCollection".to_string(),
            features,
        }
    }

    /// Convert a specific timezone's preindex data to GeoJSON format.
    ///
    /// Returns `Some(FeatureItem)` if the timezone is found in the preindex, `None` otherwise.
    ///
    /// # Arguments
    ///
    /// * `timezone_name` - The timezone name to export (e.g., "Asia/Tokyo")
    ///
    /// # Example
    ///
    /// ```rust
    /// use tzf_rs::FuzzyFinder;
    ///
    /// let finder = FuzzyFinder::new();
    /// if let Some(feature) = finder.get_tz_geojson("Asia/Tokyo") {
    ///     let json_string = feature.to_string();
    ///     println!("Found {} tiles for timezone", feature.geometry.coordinates.len());
    /// }
    /// ```
    #[must_use]
    #[cfg(feature = "export-geojson")]
    pub fn get_tz_geojson(&self, timezone_name: &str) -> Option<FeatureItem> {
        // The name table is sorted, so binary search for the index.
        let target = u16::try_from(
            self.names
                .binary_search_by(|name| name.as_str().cmp(timezone_name))
                .ok()?,
        )
        .ok()?;

        let mut keys = Vec::new();

        // Find all tiles that contain this timezone
        for (key, entry) in &self.all {
            if entry.indices().contains(&target) {
                keys.push(unpack_tile_key(*key));
            }
        }

        if keys.is_empty() {
            return None;
        }

        let mut multi_polygon_coords = MultiPolygonCoordinates::new();

        for (x, y, z) in keys {
            // Convert tile coordinates to lat/lng bounds
            let tile_poly = tile_to_polygon(x, y, z);
            multi_polygon_coords.push(vec![tile_poly]);
        }

        Some(FeatureItem {
            feature_type: "Feature".to_string(),
            properties: PropertiesDefine {
                tzid: timezone_name.to_string(),
            },
            geometry: GeometryDefine {
                geometry_type: "MultiPolygon".to_string(),
                coordinates: multi_polygon_coords,
            },
        })
    }
}

/// Convert tile coordinates (x, y, z) to a polygon representing the tile bounds.
#[cfg(feature = "export-geojson")]
#[allow(clippy::cast_precision_loss)]
fn tile_to_polygon(x: i64, y: i64, z: i64) -> Vec<[f64; 2]> {
    let n = f64::powf(2.0, z as f64);

    // Calculate min (west, south) corner
    let lng_min = (x as f64) / n * 360.0 - 180.0;
    let lat_min_rad = ((1.0 - ((y + 1) as f64) / n * 2.0) * PI).sinh().atan();
    let lat_min = lat_min_rad.to_degrees();

    // Calculate max (east, north) corner
    let lng_max = ((x + 1) as f64) / n * 360.0 - 180.0;
    let lat_max_rad = ((1.0 - (y as f64) / n * 2.0) * PI).sinh().atan();
    let lat_max = lat_max_rad.to_degrees();

    // Create a closed polygon (5 points, first == last)
    vec![
        [lng_min, lat_min],
        [lng_max, lat_min],
        [lng_max, lat_max],
        [lng_min, lat_max],
        [lng_min, lat_min],
    ]
}

/// It's most recommend to use, combine both [`Finder`] and [`FuzzyFinder`],
/// if [`FuzzyFinder`] got no data, then use [`Finder`].
pub struct DefaultFinder {
    pub finder: Finder,
    pub fuzzy_finder: FuzzyFinder,
}

impl Default for DefaultFinder {
    /// Creates a new, empty `DefaultFinder`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// ```
    fn default() -> Self {
        let options = FinderOptions::y_stripes();
        let topo_bytes = load_topology_compress_topo();
        let tzs = pbgen::CompressedTopoTimezones::try_from(topo_bytes).unwrap_or_default();
        let finder = Finder::from_compressed_topo_with_options(tzs, options);

        let fuzzy_finder = FuzzyFinder::default();

        Self {
            finder,
            fuzzy_finder,
        }
    }
}

impl DefaultFinder {
    /// Creates a new `DefaultFinder` with explicit polygon build options.
    ///
    /// The selected options are applied to the internal `Finder`.
    #[must_use]
    pub fn new_with_options(options: FinderOptions) -> Self {
        let topo_bytes = load_topology_compress_topo();
        let tzs = pbgen::CompressedTopoTimezones::try_from(topo_bytes).unwrap_or_default();
        Self {
            finder: Finder::from_compressed_topo_with_options(tzs, options),
            fuzzy_finder: FuzzyFinder::default(),
        }
    }

    /// Use lossless data to create a new `DefaultFinder`.
    ///
    /// Similar to [`DefaultFinder::new`], but the internal [`Finder`] uses
    /// `combined-with-oceans.compress.topo.bin` (~17 MB, no topology simplification)
    /// instead of the default topology-simplified dataset (~5.4 MB). Higher precision, ~1 GB memory usage.
    ///
    /// Requires the `full` feature to be enabled and must use a git dependency:
    /// ```toml
    /// tzf-rs = { git = "https://github.com/ringsaturn/tzf-rs", features = ["full"], default-features = false }
    /// ```
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[cfg(feature = "full")]
    /// # {
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new_full();
    /// assert_eq!("Asia/Shanghai", finder.get_tz_name(116.3883, 39.9289));
    /// # }
    /// ```
    #[must_use]
    #[cfg(feature = "full")]
    #[cfg_attr(docsrs, doc(cfg(feature = "full")))]
    pub fn new_full() -> Self {
        Self::new_full_with_options(FinderOptions::y_stripes())
    }

    /// Creates a `DefaultFinder` using full-precision data with explicit polygon build options.
    #[must_use]
    #[cfg(feature = "full")]
    #[cfg_attr(docsrs, doc(cfg(feature = "full")))]
    pub fn new_full_with_options(options: FinderOptions) -> Self {
        let tzs =
            pbgen::CompressedTopoTimezones::try_from(load_compress_topo()).unwrap_or_default();
        Self {
            finder: Finder::from_compressed_topo_with_options(tzs, options),
            fuzzy_finder: FuzzyFinder::default(),
        }
    }

    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// assert_eq!("Asia/Shanghai", finder.get_tz_name(116.3883, 39.9289));
    /// ```
    #[must_use]
    pub fn get_tz_name(&self, lng: f64, lat: f64) -> &str {
        let fuzzy = self.fuzzy_finder.get_tz_name(lng, lat);
        if !fuzzy.is_empty() {
            return fuzzy;
        }
        self.finder.get_tz_name(lng, lat)
    }

    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// println!("{:?}", finder.get_tz_names(116.3883, 39.9289));
    /// ```
    #[must_use]
    pub fn get_tz_names(&self, lng: f64, lat: f64) -> Vec<&str> {
        self.finder.get_tz_names(lng, lat)
    }

    /// Returns all time zone names as a `Vec<&str>`.
    ///
    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// println!("{:?}", finder.timezonenames());
    /// ```
    #[must_use]
    pub fn timezonenames(&self) -> Vec<&str> {
        self.finder.timezonenames()
    }

    /// Returns the version of the data used by this `DefaultFinder` as a `&str`.
    ///
    /// Example:
    ///
    /// ```rust
    /// use tzf_rs::DefaultFinder;
    ///
    /// let finder = DefaultFinder::new();
    /// println!("{:?}", finder.data_version());
    /// ```
    #[must_use]
    pub fn data_version(&self) -> &str {
        self.finder.data_version()
    }

    /// Creates a new instance of `DefaultFinder`.
    ///
    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert the DefaultFinder's data to GeoJSON format.
    ///
    /// This uses the underlying `Finder`'s data for the GeoJSON conversion.
    ///
    /// Returns a `BoundaryFile` (FeatureCollection) containing all timezone polygons.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tzf_rs::DefaultFinder;
    ///
    /// let finder = DefaultFinder::new();
    /// let geojson = finder.to_geojson();
    /// let json_string = geojson.to_string();
    /// ```
    #[must_use]
    #[cfg(feature = "export-geojson")]
    pub fn to_geojson(&self) -> BoundaryFile {
        self.finder.to_geojson()
    }

    /// Convert a specific timezone to GeoJSON format.
    ///
    /// This uses the underlying `Finder`'s data for the GeoJSON conversion.
    ///
    /// Returns `Some(BoundaryFile)` containing a FeatureCollection with all features
    /// for the timezone if found, `None` otherwise. The returned FeatureCollection
    /// may contain multiple features if the timezone has multiple geographic boundaries.
    ///
    /// # Arguments
    ///
    /// * `timezone_name` - The timezone name to export (e.g., "Asia/Tokyo")
    ///
    /// # Example
    ///
    /// ```rust
    /// use tzf_rs::DefaultFinder;
    ///
    /// let finder = DefaultFinder::new();
    /// if let Some(collection) = finder.get_tz_geojson("Asia/Tokyo") {
    ///     let json_string = collection.to_string();
    ///     println!("Found {} feature(s)", collection.features.len());
    ///     if let Some(first_feature) = collection.features.first() {
    ///         println!("Timezone ID: {}", first_feature.properties.tzid);
    ///     }
    /// }
    /// ```
    #[must_use]
    #[cfg(feature = "export-geojson")]
    pub fn get_tz_geojson(&self, timezone_name: &str) -> Option<BoundaryFile> {
        self.finder.get_tz_geojson(timezone_name)
    }
}
