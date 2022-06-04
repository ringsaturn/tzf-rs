mod gen;
mod geometry;

#[derive(Debug)]
pub struct Item {
    pub poly: Vec<geometry::Polygon>,
    pub tz: gen::Timezone,
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
    // type Error = anyhow::Error;

    pub fn from_pb(tzs: gen::Timezones) -> Finder {
        // TODO
        Finder { all: vec![] }
    }

    // https://users.rust-lang.org/t/cannot-move-out-of-x-which-is-behind-a-shared-reference/33263
    pub fn get_tz_name(&self, p: &geometry::Point) -> &str {
        for item in self.all.iter() {
            if item.contain_point(p){
                return &item.tz.name
            }
        }
        return ""
    }
}

fn main() {
    println!("Hello, world!");

    let tz = gen::Timezones::try_from("data/combined-with-oceans.reduce.pb".to_string()).unwrap();

    println!("一共有 {:?} 个时区", tz.timezones.len());

    let poly = geometry::Polygon {
        edges: vec![
            geometry::Edge::new((0.0, 0.0), (10.0, 0.0)),
            geometry::Edge::new((10.0, 0.0), (10.0, 10.0)),
            geometry::Edge::new((10.0, 10.0), (0.0, 10.0)),
            geometry::Edge::new((0.0, 10.0), (0.0, 0.0)),
        ],
    };

    // let item = Item { poly: vec![poly], tz: None};

    println!("{:?}", poly);
}
