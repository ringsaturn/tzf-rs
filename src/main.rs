mod gen;

fn main() {
    println!("Hello, world!");

    let tz = gen::Timezones::try_from("data/combined-with-oceans.reduce.pb".to_string()).unwrap();

    println!("一共有 {:?} 个时区", tz.timezones.len());
}
