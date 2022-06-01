use std::process::Command;

fn main() {
    prost_build::Config::new()
        .out_dir("src/gen/")
        .compile_protos(&["./tzinfo.proto"], &["."])
        .unwrap();
    Command::new("cargo")
        .args(&["fmt", "--", "src/*.rs"])
        .status()
        .expect("cargo fmt failed");
}
