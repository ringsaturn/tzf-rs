//! Fast timezone finder for Rust.
//!
//! It's designed for high performance geo queries related services like
//! weather forecast API. And most queries could return in very limited time,
//! averagely like 2000 nanoseconds.
//!
//! Please note that this package use a simplified shape data so not so accurate
//! around border.
//!
//! There are there finders implements:
//! - [Finder]: works anywhere.
//! - [FuzzyFinder]: blazing fast for most places on earth, use a preindex data.
//!   Not work for places around borders.
//! - [DefaultFinder]: combine both, if [FuzzyFinder] got no data, then use [Finder].
//!
//! Preprocessed timezone data is distributed via [tzf-rel].
//!
//! It's Rust port of [tzf] and also the foundation of
//! [tzfpy] since `v0.11.0`....
//!
//! [tzf]: https://github.com/ringsaturn/tzf
//! [tzf-rel]: https://github.com/ringsaturn/tzf-rel
//! [tzfpy]: https://github.com/ringsaturn/tzfpy
//
use geometry_rs::{Point, Polygon};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::vec;
use tzf_rel::{load_preindex, load_reduced};
mod gen;

#[derive(Debug)]
struct Item {
    polys: Vec<Polygon>,
    name: String,
}

impl Item {
    fn contain_point(&self, p: &Point) -> bool {
        for poly in self.polys.iter() {
            if poly.contains_point(*p) {
                return true;
            }
        }
        return false;
    }
}

/// Finder use a fine tuned Ray casting algorithm implement [geometry-rs]
/// which is Rust port of [geometry] by [Josh Baker].
///
/// [geometry-rs]: https://github.com/ringsaturn/geometry-rs
/// [geometry]: https://github.com/tidwall/geometry
/// [Josh Baker]: https://github.com/tidwall
#[derive(Debug)]
pub struct Finder {
    all: Vec<Item>,
}

impl Finder {
    /// `from_pb` is used when you can customed timezone data, as long as
    /// it's compatible with Proto's desc.
    pub fn from_pb(tzs: gen::Timezones) -> Finder {
        let mut f: Finder = Finder { all: vec![] };
        for tz in tzs.timezones.iter() {
            let mut polys: Vec<Polygon> = vec![];

            for pbpoly in tz.polygons.iter() {
                let mut exterior: Vec<Point> = vec![];
                for pbpoint in pbpoly.points.iter() {
                    exterior.push(Point {
                        x: pbpoint.lng as f64,
                        y: pbpoint.lat as f64,
                    })
                }

                let mut interior: Vec<Vec<Point>> = vec![];

                for holepoly in pbpoly.holes.iter() {
                    let mut holeextr: Vec<Point> = vec![];
                    for holepoint in holepoly.points.iter() {
                        holeextr.push(Point {
                            x: holepoint.lng as f64,
                            y: holepoint.lat as f64,
                        })
                    }
                    interior.push(holeextr);
                }

                let geopoly = geometry_rs::Polygon::new(exterior, interior);
                polys.push(geopoly);
            }

            let item: Item = Item {
                name: tz.name.to_string(),
                polys: polys,
            };

            f.all.push(item);
        }
        return f;
    }

    /// new is for most general usacase.
    ///
    /// Example:
    ///
    /// ```rust
    /// use tzf_rs::Finder;
    ///
    /// let finder = Finder::new();
    /// ```
    pub fn new() -> Finder {
        // let file_bytes = include_bytes!("data/combined-with-oceans.reduce.pb").to_vec();
        let file_bytes: Vec<u8> = load_reduced();
        let finder: Finder = Finder::from_pb(gen::Timezones::try_from(file_bytes).unwrap());
        return finder;
    }

    /// Example:
    ///
    /// ```rust
    /// use tzf_rs::Finder;
    ///
    /// let finder = Finder::new();
    /// assert_eq!("Asia/Shanghai", finder.get_tz_name(116.3883, 39.9289));
    /// ```
    pub fn get_tz_name(&self, lng: f64, lat: f64) -> &str {
        // let p = &Point::new(lng, lat);
        let p = geometry_rs::Point { x: lng, y: lat };
        for item in self.all.iter() {
            if item.contain_point(&p) {
                return &item.name;
            }
        }
        return "";
    }

    /// Example:
    ///
    /// ```rust
    /// use tzf_rs::Finder;
    ///
    /// let finder = Finder::new();
    /// println!("{:?}", finder.timezonenames());
    /// ```
    pub fn timezonenames(&self) -> Vec<&str> {
        let mut ret: Vec<&str> = vec![];
        for item in self.all.iter() {
            ret.push(&item.name);
        }
        return ret;
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
pub fn deg2num(lng: f64, lat: f64, zoom: i64) -> (i64, i64) {
    let lat_rad = lat.to_radians();
    let n = f64::powf(2.0, zoom as f64);
    let xtile = (lng + 180.0) / 360.0 * n;
    let ytile = (1.0 - lat_rad.tan().asinh() / PI) / 2.0 * n;
    return (xtile as i64, ytile as i64);
}

/// FuzzyFinder store all preindex's tiles data in a HashMap,
/// It iterate all zoom levels for input's longitude and latitude to build
/// map key to to check if in map.
///
/// It's is very fast and use about 400ns to check if has preindex.
/// It work for most places on earch and here is a quick loop of preindex data:
/// ![](https://user-images.githubusercontent.com/13536789/200174943-7d40661e-bda5-4b79-a867-ec637e245a49.png)
///
#[derive(Debug)]
pub struct FuzzyFinder {
    min_zoom: i64,
    max_zoom: i64,
    all: HashMap<(i64, i64, i64), String>, // K: <x,y,z>
}

impl FuzzyFinder {
    pub fn from_pb(tzs: gen::PreindexTimezones) -> FuzzyFinder {
        let mut f = FuzzyFinder {
            min_zoom: tzs.agg_zoom as i64,
            max_zoom: tzs.idx_zoom as i64,
            all: HashMap::new(),
        };
        for item in tzs.keys.iter() {
            f.all.insert(
                (item.x as i64, item.y as i64, item.z as i64),
                item.name.to_string(),
            );
        }
        return f;
    }
    /// ```rust
    /// use tzf_rs::FuzzyFinder;
    ///
    /// let finder = FuzzyFinder::new();
    /// ```
    pub fn new() -> FuzzyFinder {
        let file_bytes: Vec<u8> = load_preindex();
        let finder: FuzzyFinder =
            FuzzyFinder::from_pb(gen::PreindexTimezones::try_from(file_bytes).unwrap());
        return finder;
    }

    /// Example:
    ///
    /// ```rust
    /// use tzf_rs::FuzzyFinder;
    ///
    /// let finder = FuzzyFinder::new();
    /// assert_eq!("Asia/Shanghai", finder.get_tz_name(116.3883, 39.9289));
    /// ```
    pub fn get_tz_name(&self, lng: f64, lat: f64) -> &str {
        for zoom in self.min_zoom..self.max_zoom {
            let idx = deg2num(lng, lat, zoom);
            let k = &(idx.0, idx.1, zoom);
            let ret = self.all.get(&k);
            if ret.is_none() {
                continue;
            }
            return ret.unwrap();
        }
        return "";
    }
}

/// It's most recommend to use, combine both [Finder] and [FuzzyFinder].
pub struct DefaultFinder {
    finder: Finder,
    fuzzy_finder: FuzzyFinder,
}

impl DefaultFinder {
    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// ```
    pub fn new() -> DefaultFinder {
        let finder = Finder::new();
        let fuzzy_finder = FuzzyFinder::new();
        let df = DefaultFinder {
            finder,
            fuzzy_finder,
        };
        return df;
    }

    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// assert_eq!("Asia/Shanghai", finder.get_tz_name(116.3883, 39.9289));
    /// ```
    pub fn get_tz_name(&self, lng: f64, lat: f64) -> &str {
        let fuzzy_name = self.fuzzy_finder.get_tz_name(lng, lat);
        if fuzzy_name != "" {
            return fuzzy_name;
        }
        return self.finder.get_tz_name(lng, lat);
    }

    /// ```rust
    /// use tzf_rs::DefaultFinder;
    /// let finder = DefaultFinder::new();
    /// println!("{:?}", finder.timezonenames());
    /// ```
    pub fn timezonenames(&self) -> Vec<&str> {
        return self.finder.timezonenames();
    }
}
