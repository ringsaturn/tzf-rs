#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

use geometry_rs::{Point, Polygon};
#[cfg(feature = "export-geojson")]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::vec;
use tzf_rel::{load_preindex, load_reduced};
pub mod pbgen;

struct Item {
    polys: Vec<Polygon>,
    name: String,
}

impl Item {
    fn contains_point(&self, p: &Point) -> bool {
        for poly in &self.polys {
            if poly.contains_point(*p) {
                return true;
            }
        }
        false
    }
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
    all: Vec<Item>,
    data_version: String,
}

impl Finder {
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
        let mut f = Self {
            all: vec![],
            data_version: tzs.version,
        };
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

                let geopoly = geometry_rs::Polygon::new(exterior, interior);
                polys.push(geopoly);
            }

            let item: Item = Item {
                name: tz.name.to_string(),
                polys,
            };

            f.all.push(item);
        }
        f
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
        let direct_res = self._get_tz_name(lng, lat);
        if !direct_res.is_empty() {
            return direct_res;
        }

        for &dx in &[0.0, -0.01, 0.01, -0.02, 0.02] {
            for &dy in &[0.0, -0.01, 0.01, -0.02, 0.02] {
                let dlng = dx + lng;
                let dlat = dy + lat;
                let name = self._get_tz_name(dlng, dlat);
                if !name.is_empty() {
                    return name;
                }
            }
        }
        ""
    }

    fn _get_tz_name(&self, lng: f64, lat: f64) -> &str {
        let p = geometry_rs::Point { x: lng, y: lat };
        for item in &self.all {
            if item.contains_point(&p) {
                return &item.name;
            }
        }
        ""
    }

    /// ```rust
    /// use tzf_rs::Finder;
    /// let finder = Finder::new();
    /// println!("{:?}", finder.get_tz_names(116.3883, 39.9289));
    /// ```
    #[must_use]
    pub fn get_tz_names(&self, lng: f64, lat: f64) -> Vec<&str> {
        let mut ret: Vec<&str> = vec![];
        let p = geometry_rs::Point { x: lng, y: lat };
        for item in &self.all {
            if item.contains_point(&p) {
                ret.push(&item.name);
            }
        }
        ret
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
        let mut ret: Vec<&str> = vec![];
        for item in &self.all {
            ret.push(&item.name);
        }
        ret
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
        &self.data_version
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

    /// Helper method to convert an Item to a FeatureItem.
    #[cfg(feature = "export-geojson")]
    fn item_to_feature(&self, item: &Item) -> FeatureItem {
        // Convert internal Item to pbgen::Timezone format
        let mut pbpolys = Vec::new();
        for poly in &item.polys {
            let mut pbpoly = pbgen::Polygon {
                points: Vec::new(),
                holes: Vec::new(),
            };

            // Convert exterior points
            for point in &poly.exterior {
                pbpoly.points.push(pbgen::Point {
                    lng: point.x as f32,
                    lat: point.y as f32,
                });
            }

            // Convert holes
            for hole in &poly.holes {
                let mut hole_poly = pbgen::Polygon {
                    points: Vec::new(),
                    holes: Vec::new(),
                };
                for point in hole {
                    hole_poly.points.push(pbgen::Point {
                        lng: point.x as f32,
                        lat: point.y as f32,
                    });
                }
                pbpoly.holes.push(hole_poly);
            }

            pbpolys.push(pbpoly);
        }

        let pbtz = pbgen::Timezone {
            polygons: pbpolys,
            name: item.name.clone(),
        };

        revert_item(&pbtz)
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
        let mut output = BoundaryFile {
            collection_type: "FeatureCollection".to_string(),
            features: Vec::new(),
        };

        for item in &self.all {
            output.features.push(self.item_to_feature(item));
        }

        output
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
        // let file_bytes = include_bytes!("data/combined-with-oceans.reduce.pb").to_vec();
        let file_bytes: Vec<u8> = load_reduced();
        Self::from_pb(pbgen::Timezones::try_from(file_bytes).unwrap_or_default())
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
    let lat_rad = lat.to_radians();
    let n = f64::powf(2.0, zoom as f64);
    let xtile = (lng + 180.0) / 360.0 * n;
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
    all: HashMap<(i64, i64, i64), Vec<String>>, // K: <x,y,z>
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
        let file_bytes: Vec<u8> = load_preindex();
        Self::from_pb(pbgen::PreindexTimezones::try_from(file_bytes).unwrap_or_default())
    }
}

impl FuzzyFinder {
    #[must_use]
    pub fn from_pb(tzs: pbgen::PreindexTimezones) -> Self {
        let mut f = Self {
            min_zoom: i64::from(tzs.agg_zoom),
            max_zoom: i64::from(tzs.idx_zoom),
            all: HashMap::new(),
            data_version: tzs.version,
        };
        for item in &tzs.keys {
            let key = (i64::from(item.x), i64::from(item.y), i64::from(item.z));
            f.all.entry(key).or_insert_with(std::vec::Vec::new);
            f.all.get_mut(&key).unwrap().push(item.name.to_string());
            f.all.get_mut(&key).unwrap().sort();
        }
        f
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
        for zoom in self.min_zoom..self.max_zoom {
            let idx = deg2num(lng, lat, zoom);
            let k = &(idx.0, idx.1, zoom);
            let ret = self.all.get(k);
            if ret.is_none() {
                continue;
            }
            return ret.unwrap().first().unwrap();
        }
        ""
    }

    pub fn get_tz_names(&self, lng: f64, lat: f64) -> Vec<&str> {
        let mut names: Vec<&str> = vec![];
        for zoom in self.min_zoom..self.max_zoom {
            let idx = deg2num(lng, lat, zoom);
            let k = &(idx.0, idx.1, zoom);
            let ret = self.all.get(k);
            if ret.is_none() {
                continue;
            }
            for item in ret.unwrap() {
                names.push(item);
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
        let mut name_to_keys: HashMap<&String, Vec<(i64, i64, i64)>> = HashMap::new();

        // Group tiles by timezone name
        for (key, names) in &self.all {
            for name in names {
                name_to_keys.entry(name).or_insert_with(Vec::new).push(*key);
            }
        }

        let mut features = Vec::new();

        for (name, keys) in name_to_keys {
            let mut multi_polygon_coords = MultiPolygonCoordinates::new();

            for (x, y, z) in keys {
                // Convert tile coordinates to lat/lng bounds
                let tile_poly = tile_to_polygon(x, y, z);
                multi_polygon_coords.push(vec![tile_poly]);
            }

            let feature = FeatureItem {
                feature_type: "Feature".to_string(),
                properties: PropertiesDefine { tzid: name.clone() },
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
        let mut keys = Vec::new();

        // Find all tiles that contain this timezone
        for (key, names) in &self.all {
            if names.iter().any(|n| n == timezone_name) {
                keys.push(*key);
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
        let finder = Finder::default();
        let fuzzy_finder = FuzzyFinder::default();

        Self {
            finder,
            fuzzy_finder,
        }
    }
}

impl DefaultFinder {
    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// assert_eq!("Asia/Shanghai", finder.get_tz_name(116.3883, 39.9289));
    /// ```
    #[must_use]
    pub fn get_tz_name(&self, lng: f64, lat: f64) -> &str {
        // The simplified polygon data contains some empty areas where not covered by any timezone.
        // It's not a bug but a limitation of the simplified algorithm.
        //
        // To handle this, auto shift the point a little bit to find the nearest timezone.
        let res = self.get_tz_names(lng, lat);
        if !res.is_empty() {
            return res.first().unwrap();
        }
        ""
    }

    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// println!("{:?}", finder.get_tz_names(116.3883, 39.9289));
    /// ```
    #[must_use]
    pub fn get_tz_names(&self, lng: f64, lat: f64) -> Vec<&str> {
        for &dx in &[0.0, -0.01, 0.01, -0.02, 0.02] {
            for &dy in &[0.0, -0.01, 0.01, -0.02, 0.02] {
                let dlng = dx + lng;
                let dlat = dy + lat;
                let fuzzy_names = self.fuzzy_finder.get_tz_names(dlng, dlat);
                if !fuzzy_names.is_empty() {
                    return fuzzy_names;
                }
                let names = self.finder.get_tz_names(dlng, dlat);
                if !names.is_empty() {
                    return names;
                }
            }
        }
        Vec::new() // Return empty vector if no timezone is found
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
        &self.finder.data_version
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
