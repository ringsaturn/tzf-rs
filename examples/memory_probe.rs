/// Constructs one finder mechanism and idles briefly so an external tool
/// (`/usr/bin/time -l` on macOS, `/usr/bin/time -v` on Linux) can record the
/// process's peak RSS.
///
///   /usr/bin/time -l cargo run --release --example memory_probe -- default
use tzf_rs::{DefaultFinder, EmbeddedFinder};

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "default".into());
    let name = match mode.as_str() {
        "embedded" => {
            let f = EmbeddedFinder::new();
            f.get_tz_name(116.3883, 39.9289).to_string()
        }
        #[cfg(feature = "full")]
        "full" => {
            let f = DefaultFinder::new_full();
            f.get_tz_name(116.3883, 39.9289).to_string()
        }
        _ => {
            let f = DefaultFinder::new();
            f.get_tz_name(116.3883, 39.9289).to_string()
        }
    };
    println!("{mode}: {name}");
}
