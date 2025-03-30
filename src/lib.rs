#![doc = include_str!("../README.md")]

use geometry_rs::{Point, Polygon};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::vec;
use tzf_rel::{load_preindex, load_reduced};
pub mod gen;

// GeoJSON related structs
#[derive(Debug, serde::Serialize)]
pub struct GeoJSONFeatureCollection {
    r#type: String,
    features: Vec<GeoJSONFeature>,
}

#[derive(Debug, serde::Serialize)]
pub struct GeoJSONFeature {
    r#type: String,
    properties: GeoJSONProperties,
    geometry: GeoJSONGeometry,
}

#[derive(Debug, serde::Serialize)]
pub struct GeoJSONProperties {
    tzid: String,
}

#[derive(Debug, serde::Serialize)]
pub struct GeoJSONGeometry {
    r#type: String,
    coordinates: Vec<Vec<Vec<[f64; 2]>>>,
}

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
    pub fn from_pb(tzs: gen::Timezones) -> Self {
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

    /// Convert timezone data to GeoJSON format
    /// 
    /// # Returns
    /// 
    /// A GeoJSON FeatureCollection containing all timezone boundaries
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use tzf_rs::Finder;
    /// let finder = Finder::new();
    /// let geojson = finder.to_geojson();
    /// ```
    #[must_use]
    pub fn to_geojson(&self) -> GeoJSONFeatureCollection {
        let mut features = Vec::new();
        
        for item in &self.all {
            let mut coordinates = Vec::new();
            
            for poly in &item.polys {
                let mut polygon = Vec::new();
                
                // Add exterior ring
                let mut exterior_coords = Vec::new();
                for point in &poly.exterior {
                    exterior_coords.push([point.x, point.y]);
                }
                // Close the ring by adding the first point again
                if let Some(first) = exterior_coords.first() {
                    exterior_coords.push([first[0], first[1]]);
                }
                polygon.push(exterior_coords);
                
                // Add interior rings (holes)
                for hole in &poly.holes {
                    let mut interior_coords = Vec::new();
                    for point in hole {
                        interior_coords.push([point.x, point.y]);
                    }
                    // Close the ring
                    if let Some(first) = interior_coords.first() {
                        interior_coords.push([first[0], first[1]]);
                    }
                    polygon.push(interior_coords);
                }
                
                coordinates.push(polygon);
            }
            
            let feature = GeoJSONFeature {
                r#type: "Feature".to_string(),
                properties: GeoJSONProperties {
                    tzid: item.name.clone(),
                },
                geometry: GeoJSONGeometry {
                    r#type: "MultiPolygon".to_string(),
                    coordinates,
                },
            };
            
            features.push(feature);
        }
        
        GeoJSONFeatureCollection {
            r#type: "FeatureCollection".to_string(),
            features,
        }
    }

    /// Convert a specific timezone to GeoJSON format
    /// 
    /// # Arguments
    /// 
    /// * `timezone` - The timezone name to convert
    /// 
    /// # Returns
    /// 
    /// A GeoJSON Feature containing the timezone boundary, or None if timezone not found
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use tzf_rs::Finder;
    /// let finder = Finder::new();
    /// let geojson = finder.timezone_to_geojson("Asia/Shanghai");
    /// ```
    #[must_use]
    pub fn timezone_to_geojson(&self, timezone: &str) -> Option<GeoJSONFeature> {
        for item in &self.all {
            if item.name == timezone {
                let mut coordinates = Vec::new();
                
                for poly in &item.polys {
                    let mut polygon = Vec::new();
                    
                    // Add exterior ring
                    let mut exterior_coords = Vec::new();
                    for point in &poly.exterior {
                        exterior_coords.push([point.x, point.y]);
                    }
                    // Close the ring by adding the first point again
                    if let Some(first) = exterior_coords.first() {
                        exterior_coords.push([first[0], first[1]]);
                    }
                    polygon.push(exterior_coords);
                    
                    // Add interior rings (holes)
                    for hole in &poly.holes {
                        let mut interior_coords = Vec::new();
                        for point in hole {
                            interior_coords.push([point.x, point.y]);
                        }
                        // Close the ring
                        if let Some(first) = interior_coords.first() {
                            interior_coords.push([first[0], first[1]]);
                        }
                        polygon.push(interior_coords);
                    }
                    
                    coordinates.push(polygon);
                }
                
                return Some(GeoJSONFeature {
                    r#type: "Feature".to_string(),
                    properties: GeoJSONProperties {
                        tzid: timezone.to_string(),
                    },
                    geometry: GeoJSONGeometry {
                        r#type: "MultiPolygon".to_string(),
                        coordinates,
                    },
                });
            }
        }
        None
    }

    /// Get polygon information for a specific timezone
    /// 
    /// # Arguments
    /// 
    /// * `timezone` - The timezone name to get information for
    /// 
    /// # Returns
    /// 
    /// A tuple containing the number of polygons and a vector of tuples containing exterior points count and holes count
    #[must_use]
    pub fn get_timezone_info(&self, timezone: &str) -> Option<(usize, Vec<(usize, usize)>)> {
        for item in &self.all {
            if item.name == timezone {
                let poly_info: Vec<(usize, usize)> = item.polys
                    .iter()
                    .map(|p| (p.exterior.len(), p.holes.len()))
                    .collect();
                return Some((item.polys.len(), poly_info));
            }
        }
        None
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
        Self::from_pb(gen::Timezones::try_from(file_bytes).unwrap_or_default())
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
        Self::from_pb(gen::PreindexTimezones::try_from(file_bytes).unwrap_or_default())
    }
}

impl FuzzyFinder {
    #[must_use]
    pub fn from_pb(tzs: gen::PreindexTimezones) -> Self {
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

    /// Convert preindex timezone data to GeoJSON format
    /// 
    /// # Returns
    /// 
    /// A GeoJSON FeatureCollection containing all timezone tile boundaries
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use tzf_rs::FuzzyFinder;
    /// let finder = FuzzyFinder::new();
    /// let geojson = finder.to_geojson();
    /// ```
    #[must_use]
    pub fn to_geojson(&self) -> GeoJSONFeatureCollection {
        let mut name_to_tiles: HashMap<String, Vec<(i64, i64, i64)>> = HashMap::new();
        
        // Group tiles by timezone name
        for (&(x, y, z), names) in &self.all {
            for name in names {
                name_to_tiles
                    .entry(name.clone())
                    .or_default()
                    .push((x, y, z));
            }
        }
        
        let mut features = Vec::new();
        
        for (name, tiles) in name_to_tiles {
            let mut coordinates = Vec::new();
            
            for (x, y, z) in tiles {
                // Convert tile coordinates to lat/lon bounds
                let n = f64::powf(2.0, z as f64);
                let lon_min = (x as f64 / n) * 360.0 - 180.0;
                let lon_max = ((x + 1) as f64 / n) * 360.0 - 180.0;
                let lat_rad_max = PI * (1.0 - 2.0 * y as f64 / n);
                let lat_rad_min = PI * (1.0 - 2.0 * (y + 1) as f64 / n);
                let lat_min = lat_rad_min.sinh().atan().to_degrees();
                let lat_max = lat_rad_max.sinh().atan().to_degrees();
                
                // Create a polygon for this tile
                let polygon = vec![vec![
                    [lon_min, lat_min],
                    [lon_max, lat_min],
                    [lon_max, lat_max],
                    [lon_min, lat_max],
                    [lon_min, lat_min], // Close the ring
                ]];
                
                coordinates.push(polygon);
            }
            
            let feature = GeoJSONFeature {
                r#type: "Feature".to_string(),
                properties: GeoJSONProperties {
                    tzid: name,
                },
                geometry: GeoJSONGeometry {
                    r#type: "MultiPolygon".to_string(),
                    coordinates,
                },
            };
            
            features.push(feature);
        }
        
        GeoJSONFeatureCollection {
            r#type: "FeatureCollection".to_string(),
            features,
        }
    }

    /// Convert a specific timezone to GeoJSON format
    /// 
    /// # Arguments
    /// 
    /// * `timezone` - The timezone name to convert
    /// 
    /// # Returns
    /// 
    /// A GeoJSON Feature containing the timezone boundary, or None if timezone not found
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use tzf_rs::FuzzyFinder;
    /// let finder = FuzzyFinder::new();
    /// let geojson = finder.timezone_to_geojson("Asia/Shanghai");
    /// ```
    #[must_use]
    pub fn timezone_to_geojson(&self, timezone: &str) -> Option<GeoJSONFeature> {
        let mut coordinates = Vec::new();
        let mut found = false;
        
        // Group tiles by timezone name
        for (&(x, y, z), names) in &self.all {
            if names.contains(&timezone.to_string()) {
                found = true;
                // Convert tile coordinates to lat/lon bounds
                let n = f64::powf(2.0, z as f64);
                let lon_min = (x as f64 / n) * 360.0 - 180.0;
                let lon_max = ((x + 1) as f64 / n) * 360.0 - 180.0;
                let lat_rad_max = PI * (1.0 - 2.0 * y as f64 / n);
                let lat_rad_min = PI * (1.0 - 2.0 * (y + 1) as f64 / n);
                let lat_min = lat_rad_min.sinh().atan().to_degrees();
                let lat_max = lat_rad_max.sinh().atan().to_degrees();
                
                // Create a polygon for this tile
                let polygon = vec![vec![
                    [lon_min, lat_min],
                    [lon_max, lat_min],
                    [lon_max, lat_max],
                    [lon_min, lat_max],
                    [lon_min, lat_min], // Close the ring
                ]];
                
                coordinates.push(polygon);
            }
        }
        
        if found {
            Some(GeoJSONFeature {
                r#type: "Feature".to_string(),
                properties: GeoJSONProperties {
                    tzid: timezone.to_string(),
                },
                geometry: GeoJSONGeometry {
                    r#type: "MultiPolygon".to_string(),
                    coordinates,
                },
            })
        } else {
            None
        }
    }
}

/// It's most recommend to use, combine both [`Finder`] and [`FuzzyFinder`],
/// if [`FuzzyFinder`] got no data, then use [`Finder`].
pub struct DefaultFinder {
    finder: Finder,
    fuzzy_finder: FuzzyFinder,
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

    /// Convert timezone data to GeoJSON format using the underlying Finder
    /// 
    /// # Returns
    /// 
    /// A GeoJSON FeatureCollection containing all timezone boundaries
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// let geojson = finder.to_geojson();
    /// ```
    #[must_use]
    pub fn to_geojson(&self) -> GeoJSONFeatureCollection {
        self.finder.to_geojson()
    }

    /// Convert a specific timezone to GeoJSON format using the underlying Finder
    /// 
    /// # Arguments
    /// 
    /// * `timezone` - The timezone name to convert
    /// 
    /// # Returns
    /// 
    /// A GeoJSON Feature containing the timezone boundary, or None if timezone not found
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// let geojson = finder.timezone_to_geojson("Asia/Shanghai");
    /// ```
    #[must_use]
    pub fn timezone_to_geojson(&self, timezone: &str) -> Option<GeoJSONFeature> {
        self.finder.timezone_to_geojson(timezone)
    }
}
