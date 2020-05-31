#[macro_use]
extern crate log;

use std::env;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Error, Result};

struct Library {
    pub libs: Vec<String>,
    pub link_paths: Vec<PathBuf>,
    pub include_paths: Vec<PathBuf>,
}

fn find_hyperscan() -> Result<Library> {
    env::var("HYPERSCAN_ROOT")
        .with_context(|| "HYPERSCAN_ROOT")
        .and_then(|prefix| {
            debug!("building with Hyperscan @ HYPERSCAN_ROOT={}", prefix);

            let mut libs = vec![];
            let mut link_paths = vec![];
            let mut include_paths = vec![];

            let prefix = Path::new(&prefix);
            let inc_path = prefix.join("include/hs");
            let lib_path = prefix.join("lib");
            let libhs = lib_path.join("libhs.a");
            let libchimera = lib_path.join("libchimera.a");

            if !prefix.exists() || !prefix.is_dir() {
                bail!("HYPERSCAN_ROOT should point to a directory that exists.");
            }
            if lib_path.exists() && lib_path.is_dir() {
                link_paths.push(lib_path.into())
            } else {
                bail!("`$HYPERSCAN_ROOT/lib` subdirectory not found.");
            }
            if inc_path.exists() && inc_path.is_dir() {
                include_paths.push(inc_path.into())
            } else {
                bail!("`$HYPERSCAN_ROOT/include/hs` subdirectory not found.");
            }
            if libhs.exists() && libhs.is_file() {
                libs.push("hs".into());
            } else {
                bail!("`$HYPERSCAN_ROOT/lib/libhs.a` library not found.");
            }
            if libchimera.exists() && libchimera.is_file() {
                libs.push("chimera".into());
            } else if cfg!(feature = "chimera") {
                bail!("`$HYPERSCAN_ROOT/lib/libchimera.a` library not found.");
            }

            Ok(Library {
                libs,
                link_paths,
                include_paths,
            })
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
        .map_err(|_| Error::msg("please download and install hyperscan from https://www.hyperscan.io/"))
}

#[cfg(feature = "gen")]
fn generate_binding(inc_dir: &Path, out_dir: &Path) -> Result<()> {
    let out_file = out_dir.join("raw.rs");

    info!("generating raw Hyperscan wrapper @ {}", out_file.display());

    let inc_file = inc_dir.join("hs.h");
    let inc_file = inc_file.to_str().expect("header file");

    println!("cargo:rerun-if-changed={}", inc_file);

    bindgen::builder()
        .header(inc_file)
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
        .map_err(|_| Error::msg("generate binding files"))?
        .write_to_file(out_file)
        .with_context(|| "write wrapper")
}

#[cfg(not(feature = "gen"))]
fn generate_binding(_: &Path, out_dir: &Path) -> Result<()> {
    std::fs::copy("src/raw.rs", out_dir.join("raw.rs"))
        .map(|_| ())
        .with_context(|| "copy binding file")
}

#[cfg(all(feature = "gen", feature = "chimera"))]
fn generate_chimera_binding(inc_dir: &Path, out_dir: &Path) -> Result<()> {
    let out_file = out_dir.join("chimera.rs");
    let inc_file = inc_dir.join("ch.h");
    let inc_file = inc_file.to_str().expect("header file");

    println!("cargo:rerun-if-changed={}", inc_file);

    bindgen::builder()
        .header(inc_file)
        .use_core()
        .ctypes_prefix("::libc")
        .clang_args(&["-x", "c++", "-std=c++11"])
        .whitelist_var("^CH_.*")
        .whitelist_type("^ch_.*")
        .whitelist_function("^ch_.*")
        .blacklist_type("^__darwin_.*")
        .size_t_is_usize(true)
        .derive_copy(false)
        .derive_default(true)
        .generate()
        .map_err(|_| Error::msg("generate binding files"))?
        .write_to_file(out_file)
        .with_context(|| "write wrapper")
}

#[cfg(all(not(feature = "gen"), feature = "chimera"))]
fn generate_chimera_binding(_: &Path, out_dir: &Path) -> Result<()> {
    std::fs::copy("src/chimera.rs", out_dir.join("chimera.rs"))
        .map(|_| ())
        .with_context(|| "copy binding file")
}

fn main() -> Result<()> {
    pretty_env_logger::init();

    let libhs = find_hyperscan()?;
    let out_dir = env::var("OUT_DIR")?;
    let out_dir = Path::new(&out_dir);
    let inc_dir = libhs.include_paths.first().expect("include path");

    generate_binding(inc_dir, &out_dir)?;

    for lib in libhs.libs {
        println!("cargo:rustc-link-lib=dylib={}", lib);
    }

    for link_path in libhs.link_paths {
        println!(
            "cargo:rustc-link-search=native={}",
            link_path.to_str().expect("link path")
        );
    }

    if cfg!(feature = "chimera") {
        generate_chimera_binding(inc_dir, &out_dir)?;
    }

    Ok(())
}
