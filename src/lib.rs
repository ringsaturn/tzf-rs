#![doc = include_str!("../README.md")]

use geo::{Contains, Coord, LineString, MultiPolygon, Point, Polygon};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::vec;
use tzf_rel::{load_preindex, load_reduced};
pub mod gen;

struct Item {
    mpoly: MultiPolygon,
    name: String,
}

impl Item {
    fn contains_point(&self, p: &Point) -> bool {
        self.mpoly.contains(p)
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
    pub fn from_pb(tzs: gen::Timezones) -> Finder {
        let mut f: Finder = Finder {
            all: vec![],
            data_version: tzs.version.to_string(),
        };
        for tz in tzs.timezones.iter() {
            let mut polys: Vec<Polygon> = vec![];

            for pbpoly in tz.polygons.iter() {
                let mut exterior: Vec<Coord> = vec![];
                for pbpoint in pbpoly.points.iter() {
                    exterior.push(Coord {
                        x: pbpoint.lng as f64,
                        y: pbpoint.lat as f64,
                    })
                }
                let mut interior: Vec<LineString> = vec![];

                for holepoly in pbpoly.holes.iter() {
                    let mut holeextr: Vec<Coord> = vec![];
                    for holepoint in holepoly.points.iter() {
                        holeextr.push(Coord {
                            x: holepoint.lng as f64,
                            y: holepoint.lat as f64,
                        })
                    }
                    interior.push(LineString::new(holeextr));
                }

                let geopoly = Polygon::new(LineString::new(exterior), interior);
                polys.push(geopoly);
            }

            let mpoly = MultiPolygon::new(polys);

            let item: Item = Item {
                name: tz.name.to_string(),
                mpoly,
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
        let p = &Point::new(lng, lat);
        for item in self.all.iter() {
            if item.contains_point(p) {
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
        let p = &Point::new(lng, lat);
        for item in &self.all {
            if item.contains_point(p) {
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
        for &dx in &[0.0, -0.001, 0.001] {
            for &dy in &[0.0, -0.001, 0.001] {
                let dlng = dx + lng;
                let dlat = dy + lat;
                let fuzzy_name = self.fuzzy_finder.get_tz_name(dlng, dlat);
                if !fuzzy_name.is_empty() {
                    return fuzzy_name;
                }
                let name = self.finder.get_tz_name(dlng, dlat);
                if !name.is_empty() {
                    return name;
                }
            }
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
}
