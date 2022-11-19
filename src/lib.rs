use std::collections::HashMap;
use std::f64::consts::PI;
use std::vec;
use tzf_rel::{load_preindex, load_reduced};
mod gen;
mod geometry;

#[derive(Debug)]
pub struct Item {
    pub poly: Vec<geometry::Polygon>,
    pub name: String,
}

impl Item {
    pub fn contain_point(&self, p: &geometry::Point) -> bool {
        for poly in self.poly.iter() {
            if geometry::pt_in_polygon(p, &poly) {
                return true;
            }
        }
        false
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
            let mut poly: Vec<geometry::Polygon> = vec![];

            // iter all polygons
            for polygon in tz.polygons.iter() {
                let mut edges: Vec<geometry::Edge> = vec![];

                let points_len = polygon.points.len() - 1;
                for i in 0..points_len {
                    let curr = polygon.points.get(i).unwrap();
                    let mut next_idx = i + 1;
                    if next_idx > points_len {
                        next_idx = 0;
                    }
                    let next = polygon.points.get(next_idx).unwrap();

                    edges.push(geometry::Edge {
                        pt1: (geometry::Point {
                            x: f64::from(curr.lng),
                            y: f64::from(curr.lat),
                        }),
                        pt2: (geometry::Point {
                            x: f64::from(next.lng),
                            y: f64::from(next.lat),
                        }),
                    })
                }

                // TODO(ringsaturn): support holes

                let newpoly: geometry::Polygon = geometry::Polygon { edges: edges };
                poly.push(newpoly);
            }

            let item: Item = Item {
                name: tz.name.to_string(),
                poly: poly,
            };

            f.all.push(item);
        }
        return f;
    }

    pub fn new_default() -> Finder {
        // let file_bytes = include_bytes!("data/combined-with-oceans.reduce.pb").to_vec();
        let file_bytes: Vec<u8> = load_reduced();
        let finder: Finder = Finder::from_pb(gen::Timezones::try_from(file_bytes).unwrap());
        return finder;
    }

    // https://users.rust-lang.org/t/cannot-move-out-of-x-which-is-behind-a-shared-reference/33263
    pub fn get_tz_name(&self, lng: f64, lat: f64) -> &str {
        let p = &geometry::Point { x: lng, y: lat };
        for item in self.all.iter() {
            if item.contain_point(p) {
                return &item.name;
            }
        }
        return "";
    }
}

// https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames
pub fn deg2num(lng: f64, lat: f64, zoom: i64) -> (i64, i64) {
    let lng_rad = lng.to_radians();
    let lat_rad = lat.to_radians();
    let n = (2 ^ zoom) as f64;
    let xtile = ((lng_rad + 180.0) / (360.0 * n)) as i64;
    let ytile = ((1.0 - lat_rad.tan().asinh() / PI) / (2.0 * n)) as i64;
    return (xtile, ytile);
}

#[derive(Debug)]
pub struct FuzzyFinder {
    min_zoom: i64,
    max_zoom: i64,
    all: HashMap<(i64, i64, i64), String>, // x,y,z
}

impl FuzzyFinder {
    pub fn from_pb(tzs: gen::PreindexTimezones) -> FuzzyFinder {
        let mut f = FuzzyFinder {
            min_zoom: tzs.idx_zoom as i64,
            max_zoom: tzs.agg_zoom as i64,
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

    pub fn new_default() -> FuzzyFinder {
        let file_bytes: Vec<u8> = load_preindex();
        let finder: FuzzyFinder =
            FuzzyFinder::from_pb(gen::PreindexTimezones::try_from(file_bytes).unwrap());
        return finder;
    }

    pub fn get_tz_name(&self, lng: f64, lat: f64) -> &str {
        for zoom in self.min_zoom..self.max_zoom {
            print!("{} ", zoom);
            let idx = deg2num(lng, lat, zoom);
            let ret = self.all.get(&(idx.0, idx.1, zoom));
            if ret.is_none() {
                continue;
            }
            return ret.unwrap();
        }
        return "";
    }
}
