use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=TZF_RS_BUILD_PB");
    match env::var_os("TZF_RS_BUILD_PB") {
        Some(_) => prost_build::Config::new()
            .out_dir("src/pbgen/")
            .compile_protos(&["./pb/tzf/v1/tzinfo.proto"], &["."])
            .unwrap_or_default(),
        None => println!("no need for pb"),
    }
}
