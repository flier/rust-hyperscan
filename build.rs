#[macro_use]
extern crate log;
extern crate libbindgen;
extern crate env_logger;

use std::env;
use std::path::Path;

fn main() {
    env_logger::init().unwrap();

    let hyperscan_root = match env::var("HYPERSCAN_ROOT") {
        Ok(prefix) => prefix,
        Err(_) => String::from("/usr/local"),
    };

    debug!("building with Hyperscan @ {}", hyperscan_root);

    let out_dir = env::var("OUT_DIR").unwrap();

    info!("generating raw Hyperscan wrapper {} ...", out_dir);

    libbindgen::builder()
        .header(format!("{}/include/hs/hs.h", hyperscan_root))
        .no_unstable_rust()
        .whitelisted_function("^hs_.*")
        .generate()
        .expect("Fail to generate bindings")
        .write_to_file(Path::new(&out_dir).join("raw.rs"))
        .expect("Fail to write raw wrapper");

    println!("cargo:rerun-if-changed={}/include/hs/hs.h", hyperscan_root);

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    println!("cargo:rustc-link-lib=static=hs");
    println!("cargo:rustc-link-search=native={}/lib", hyperscan_root);
}
