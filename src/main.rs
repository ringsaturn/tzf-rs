#![feature(test)]

use std::vec;

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
        let file_bytes = include_bytes!("data/combined-with-oceans.reduce.pb").to_vec();

        let tz = gen::Timezones::try_from(file_bytes).unwrap();

        let finder: Finder = Finder::from_pb(tz);
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

#[cfg(test)]
mod tests {

    use super::Finder;

    #[test]
    fn add() {
        assert_eq!(1, 1,);
    }

    #[test]
    fn smoke_test() {
        let finder = Finder::new_default();

        assert_eq!(finder.get_tz_name(116.3883, 39.9289), "Asia/Shanghai");
        assert_eq!(finder.get_tz_name(121.3547, 31.1139), "Asia/Shanghai");
        assert_eq!(finder.get_tz_name(111.8674, 34.4200), "Asia/Shanghai");
        assert_eq!(finder.get_tz_name(-97.8674, 34.4200), "America/Chicago");
        assert_eq!(finder.get_tz_name(139.4382, 36.4432), "Asia/Tokyo");
        assert_eq!(finder.get_tz_name(24.5212, 50.2506), "Europe/Kyiv");
        assert_eq!(finder.get_tz_name(-0.9671, 52.0152), "Europe/London");
        assert_eq!(finder.get_tz_name(-4.5706, 46.2747), "Etc/GMT");
        assert_eq!(finder.get_tz_name(-4.5706, 46.2747), "Etc/GMT");
        assert_eq!(finder.get_tz_name(-73.7729, 38.3530), "Etc/GMT+5");
        assert_eq!(finder.get_tz_name(114.1594, 22.3173), "Asia/Hong_Kong");
    }

    extern crate test;
    use test::Bencher;

    #[bench]
    fn bench_get_tz_beijing(b: &mut Bencher) {
        let finder = Finder::new_default();
        b.iter(|| {
            let _ = finder.get_tz_name(116.3883, 39.9289);
        });
    }
}

fn main() {
    let finder = Finder::new_default();

    print!("{:?}", finder.get_tz_name(116.3883, 39.9289));
}
