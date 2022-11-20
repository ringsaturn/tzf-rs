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
    pub fn contain_point(&self, p: &Point) -> bool {
        for poly in self.polys.iter() {
            if poly.contains_point(*p) {
                return true;
            }
        }
        return false;
    }
}

#[derive(Debug)]
pub struct Finder {
    all: Vec<Item>,
}

impl Finder {
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

    pub fn new() -> Finder {
        // let file_bytes = include_bytes!("data/combined-with-oceans.reduce.pb").to_vec();
        let file_bytes: Vec<u8> = load_reduced();
        let finder: Finder = Finder::from_pb(gen::Timezones::try_from(file_bytes).unwrap());
        return finder;
    }

    // https://users.rust-lang.org/t/cannot-move-out-of-x-which-is-behind-a-shared-reference/33263
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
}

// https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames
pub fn deg2num(lng: f64, lat: f64, zoom: i64) -> (i64, i64) {
    let lat_rad = lat.to_radians();
    let n = f64::powf(2.0, zoom as f64);
    let xtile = (lng + 180.0) / 360.0 * n;
    let ytile = (1.0 - lat_rad.tan().asinh() / PI) / 2.0 * n;
    return (xtile as i64, ytile as i64);
}

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

    pub fn new() -> FuzzyFinder {
        let file_bytes: Vec<u8> = load_preindex();
        let finder: FuzzyFinder =
            FuzzyFinder::from_pb(gen::PreindexTimezones::try_from(file_bytes).unwrap());
        return finder;
    }

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

pub struct DefaultFinder {
    finder: Finder,
    fuzzy_finder: FuzzyFinder,
}

impl DefaultFinder {
    pub fn new() -> DefaultFinder {
        let finder = Finder::new();
        let fuzzy_finder = FuzzyFinder::new();
        let df = DefaultFinder {
            finder,
            fuzzy_finder,
        };
        return df;
    }

    pub fn get_tz_name(&self, lng: f64, lat: f64) -> &str {
        let fuzzy_name = self.fuzzy_finder.get_tz_name(lng, lat);
        if fuzzy_name != "" {
            return fuzzy_name;
        }
        return self.finder.get_tz_name(lng, lat);
    }
}
