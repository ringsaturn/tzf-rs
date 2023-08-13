use std::env;
use std::process::Command;

fn main() {
    match env::var_os("TZF_RS_BUILD_PB") {
        Some(_) => prost_build::Config::new()
            .out_dir("src/gen/")
            .compile_protos(&["./tzinfo.proto"], &["."])
            .unwrap_or_default(),
        None => println!("no need for pb"),
    }

    Command::new("cargo")
        .args(["fmt", "--", "src/*.rs"])
        .status()
        .expect("cargo fmt failed");
}
