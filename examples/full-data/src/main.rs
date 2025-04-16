use tzf_rs::Finder;
use tzf_rs::gen::tzf::v1::Timezones;

pub fn load_full() -> Vec<u8> {
    include_bytes!("./combined-with-oceans.bin").to_vec()
}

fn main() {
    println!("Hello, world!");
    let file_bytes: Vec<u8> = load_full();
    
    let finder = Finder::from_pb(Timezones::try_from(file_bytes).unwrap_or_default());
    let tz_name = finder.get_tz_name(139.767125, 35.681236);
    println!("tz_name: {}", tz_name);
}
