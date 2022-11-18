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
        // TODO
        let mut f: Finder = Finder { all: vec![] };
        for tz in tzs.timezones.iter() {
            let mut poly: Vec<geometry::Polygon> = vec![];

            // 遍历所有的多边形
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

    // https://users.rust-lang.org/t/cannot-move-out-of-x-which-is-behind-a-shared-reference/33263
    pub fn get_tz_name(&self, p: &geometry::Point) -> &str {
        for item in self.all.iter() {
            if item.contain_point(p) {
                return &item.name;
            }
        }
        return "";
    }
}

fn main() {
    println!("Hello, world!");

    let tz = gen::Timezones::try_from("data/combined-with-oceans.reduce.pb".to_string()).unwrap();

    println!("一共有 {:?} 个时区", tz.timezones.len());

    // let _poly = geometry::Polygon {
    //     edges: vec![
    //         geometry::Edge::new((0.0, 0.0), (10.0, 0.0)),
    //         geometry::Edge::new((10.0, 0.0), (10.0, 10.0)),
    //         geometry::Edge::new((10.0, 10.0), (0.0, 10.0)),
    //         geometry::Edge::new((0.0, 10.0), (0.0, 0.0)),
    //     ],
    // };
    let finder: Finder = Finder::from_pb(tz);

    // print!("{:?}", finder);
    print!(
        "{:?}",
        finder.get_tz_name(&geometry::Point {
            x: 116.3883,
            y: 39.9289
        })
    );

    // let item = Item { poly: vec![poly], tz: None};
    // println!("{:?}", poly);
}
