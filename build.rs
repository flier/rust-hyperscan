extern crate bindgen;
extern crate env_logger;

use std::env;
use std::process::Command;

fn macos_sdk_path() -> String {
    let output = Command::new("xcrun")
        .arg("--show-sdk-path")
        .output()
        .expect("failed to execute process");

    String::from_utf8(output.stdout).expect("fail to decode utf8")
}

fn generate(base_dir: &str) {
    let mut builder = bindgen::Builder::new(format!("{}/include/hs/hs.h", base_dir));

    builder.link("hs", bindgen::LinkType::Static)
        .builtins()
        .clang_arg(format!("-I{}/include/hs", base_dir));

    if cfg!(target_os = "macos") {
        builder.clang_arg(format!("-I{}/usr/include", macos_sdk_path()));
    }

    builder.generate()
        .expect("Failed to generate bindings")
        .write_to_file("src/raw.rs")
        .expect("Failed to write file");
}

fn main() {
    env_logger::init().unwrap();

    let root_dir = match env::var("HYPERSCAN_ROOT") {
        Ok(prefix) => prefix,
        Err(_) => String::from("/usr/local"),
    };

    generate(&root_dir);

    println!("cargo:rustc-link-lib=dylib=c++");
    println!("cargo:rustc-link-lib=static=hs");
    println!("cargo:rustc-link-search=native={}/lib", root_dir);
}
