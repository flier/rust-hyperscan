# rust-hyperscan [![travis](https://api.travis-ci.org/flier/rust-hyperscan.svg)](https://travis-ci.org/flier/rust-hyperscan) [![crate](https://img.shields.io/crates/v/hyperscan.svg)](https://crates.io/crates/hyperscan) [![docs](https://docs.rs/hyperscan/badge.svg)](https://docs.rs/crate/hyperscan/)

[Hyperscan](https://github.com/intel/hyperscan) is a high-performance regular expression matching library.

## Usage

To use, add the following line to Cargo.toml under [dependencies]:

```toml
hyperscan = "0.2"
```

## Examples

```rust
use hyperscan::prelude::*;

fn main() {
    let pattern = pattern! {"test"; CASELESS | SOM_LEFTMOST};
    let db: BlockDatabase = pattern.build().unwrap();
    let scratch = db.alloc_scratch().unwrap();
    let mut matches = vec![];

    db.scan("some test data", &scratch, |id, from, to, flags| {
        println!("found pattern #{} @ [{}, {})", id, from, to);

        matches.push(from..to);

        Matching::Continue
    }).unwrap();

    assert_eq!(matches, vec![5..9]);
}
```

## Features

### Hyperscan v5 API

Starting with Hyperscan v5.0, several new APIs and flags have been introduced.

`rust-hyperscan` uses the latest version of the API by default, providing new features such as `Literal`.

If you want to work with Hyperscan v4.x, you can disable `v5` feature at compile time.

```toml
[dependencies.hyperscan]
version = "0.2"
default-features = false
```

### Chimera API

In order to improve regular expression compatibility, Hyperscan v5.0 starts to provide a PCRE-compatible [Chimera](http://intel.github.io/hyperscan/dev-reference/chimera.html) library.

To enable `Chimera` support, you need to manually download PCRE 8.41 or above, unzip to the source directory of Hyperscan 5.x, compile and install it.

```bash
$ cd hyperscan-5.3.0
$ wget https://ftp.pcre.org/pub/pcre/pcre-8.44.tar.gz
$ tar xvf -C pcre pcre-8.44.tar.gz

$ mkdir build && cd build
$ cmake .. -DCMAKE_INSTALL_PREFIX=`pwd`
```

Then point to the hyperscan installation directory with the `HYPERSCAN_ROOT` environment variable to enable `chimera` feature.

```bash
$ HYPERSCAN_ROOT=<CMAKE_INSTALL_PREFIX> cargo test --features chimera
```
