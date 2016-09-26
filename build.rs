#[macro_use]
extern crate log;
extern crate bindgen;
extern crate env_logger;

use std::env;
use std::path::Path;
use std::process::Command;

fn macos_sdk_path() -> String {
    let output = Command::new("xcrun")
        .arg("--show-sdk-path")
        .output()
        .expect("failed to execute process");

    String::from_utf8(output.stdout).expect("fail to decode utf8")
}

fn generate_raw_wrapper(base_dir: &str, out_file:&str) {
    let mut builder = bindgen::Builder::new(format!("{}/include/hs/hs.h", base_dir));

    info!("generating raw Hyperscan wrapper {} ...", out_file);

    builder.link("hs", bindgen::LinkType::Static)
        .builtins()
        .clang_arg(format!("-I{}/include/hs", base_dir));

    if cfg!(target_os = "macos") {
        let sdk_path = macos_sdk_path();

        debug!("found macOS SDK @ {}", sdk_path);

        builder.clang_arg(format!("-I{}/usr/include", sdk_path));
    }

    builder.generate()
        .expect("Failed to generate bindings")
        .write_to_file(out_file)
        .expect("Failed to write generated file");
}

fn main() {
    env_logger::init().unwrap();

    let root_dir = match env::var("HYPERSCAN_ROOT") {
        Ok(prefix) => prefix,
        Err(_) => String::from("/usr/local"),
    };

    debug!("building with Hyperscan @ {}", root_dir);

    let out_file = Path::new("src/raw.rs");

    if !out_file.exists() {
        generate_raw_wrapper(&root_dir, out_file.to_str().unwrap());
    }

    println!("cargo:rerun-if-changed={}/include/hs/hs.h", root_dir);
    println!("cargo:rustc-link-lib=dylib=c++");
    println!("cargo:rustc-link-search=native={}/lib", root_dir);
}
