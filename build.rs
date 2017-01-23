#[macro_use]
extern crate log;
#[cfg(feature = "bindgen")]
extern crate libbindgen;
extern crate env_logger;

#[cfg(not(feature = "bindgen"))]
use std::fs;
use std::env;
use std::path::Path;

#[cfg(feature = "bindgen")]
fn generate_binding(hyperscan_root: &str, out_file: &Path) {
    info!("generating raw Hyperscan wrapper @ {}", out_file.display());

    libbindgen::builder()
        .header(format!("{}/include/hs/hs.h", hyperscan_root))
        .clang_arg("-xc++")
        .clang_arg("-std=c++11")
        .no_unstable_rust()
        .whitelisted_function("^hs_.*")
        .generate()
        .expect("Fail to generate bindings")
        .write_to_file(out_file)
        .expect("Fail to write raw wrapper");

    println!("cargo:rerun-if-changed={}/include/hs/hs.h", hyperscan_root);

}

#[cfg(not(feature = "bindgen"))]
fn generate_binding(_: &str, out_file: &Path) {
    fs::copy("src/raw_bindgen.rs", out_file).expect("fail to copy bindings");
}

fn main() {
    env_logger::init().unwrap();

    let hyperscan_root = match env::var("HYPERSCAN_ROOT") {
        Ok(prefix) => prefix,
        Err(_) => String::from("/usr/local"),
    };

    debug!("building with Hyperscan @ {}", hyperscan_root);

    let out_dir = env::var("OUT_DIR").unwrap();
    let out_file = Path::new(&out_dir).join("raw_bindgen.rs");

    generate_binding(&hyperscan_root, &out_file);

    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-lib=dylib=gcc");
    }

    println!("cargo:rustc-link-lib=static=hs");
    println!("cargo:rustc-link-search=native={}/lib", hyperscan_root);
}
