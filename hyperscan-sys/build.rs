#[macro_use]
extern crate log;

use std::env;
use std::path::{Path, PathBuf};

use failure::{err_msg, Error, ResultExt};

struct Library {
    pub libs: Vec<String>,
    pub link_paths: Vec<PathBuf>,
    pub include_paths: Vec<PathBuf>,
}

fn find_hyperscan() -> Result<Library, Error> {
    env::var("HYPERSCAN_ROOT")
        .map(|prefix| {
            debug!("building with Hyperscan @ HYPERSCAN_ROOT={}", prefix);

            Library {
                libs: vec!["hs".to_owned()],
                link_paths: vec![format!("{}/lib", prefix).into()],
                include_paths: vec![format!("{}/include/hs", prefix).into()],
            }
        })
        .or_else(|_| {
            pkg_config::Config::new().statik(true).probe("libhs").map(
                |pkg_config::Library {
                     libs,
                     link_paths,
                     include_paths,
                     ..
                 }| {
                    debug!(
                        "building with Hyperscan @ libs={:?}, link_paths={:?}, include_paths={:?}",
                        libs, link_paths, include_paths
                    );

                    Library {
                        libs,
                        link_paths,
                        include_paths,
                    }
                },
            )
        })
        .or_else(|_| {
            Err(err_msg(
                "please download and install hyperscan from https://www.hyperscan.io/",
            ))
        })
}

#[cfg(feature = "gen")]
fn generate_binding(hyperscan_include_path: &str, out_file: &Path) -> Result<(), Error> {
    info!("generating raw Hyperscan wrapper @ {}", out_file.display());

    let hyperscan_include_file = format!("{}/hs.h", hyperscan_include_path);

    println!("cargo:rerun-if-changed={}", hyperscan_include_file);

    bindgen::builder()
        .header(hyperscan_include_file)
        .use_core()
        .ctypes_prefix("::libc")
        .clang_args(&["-x", "c++", "-std=c++11"])
        .whitelist_var("^HS_.*")
        .whitelist_type("^hs_.*")
        .whitelist_function("^hs_.*")
        .blacklist_type("^__darwin_.*")
        .size_t_is_usize(true)
        .derive_copy(false)
        .derive_default(true)
        .generate()
        .map_err(|_| err_msg("generate binding files"))?
        .write_to_file(out_file)
        .context("write wrapper")?;

    Ok(())
}

#[cfg(not(feature = "gen"))]
fn generate_binding(_: &str, out_file: &Path) -> Result<(), Error> {
    std::fs::copy("src/raw.rs", out_file).context("copy binding file")?;

    Ok(())
}

fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let libhs = find_hyperscan()?;

    let out_dir = env::var("OUT_DIR")?;
    let out_file = Path::new(&out_dir).join("raw.rs");

    generate_binding(libhs.include_paths[0].to_str().unwrap(), &out_file)?;

    for lib in libhs.libs {
        if lib.contains("hs") {
            println!("cargo:rustc-link-lib=dylib={}", lib);
        }
    }

    for link_path in libhs.link_paths {
        println!("cargo:rustc-link-search=native={}", link_path.to_str().unwrap());
    }

    Ok(())
}
