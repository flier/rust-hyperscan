fn main() {
    let root_dir = "/usr/local";

    println!("cargo:rustc-flags=-l static=hs -L native={}/lib -l stdc++ -L native=/usr/lib",
             root_dir);
    println!("cargo:root={}", root_dir);
    println!("cargo:libdir={}/lib", root_dir);
    println!("cargo:include={}/include", root_dir);
}
