use std::env;

fn main() {
    let root_dir = match env::var("HYPERSCAN_ROOT") {
        Ok(prefix) => prefix,
        Err(_) => String::from("/usr/local"),
    };

    println!("cargo:rustc-flags=-L native={}/lib -l stdc++ -L native=/usr/lib",
             root_dir);
    println!("cargo:root={}", root_dir);
    println!("cargo:libdir={}/lib", root_dir);
    println!("cargo:include={}/include", root_dir);
}
