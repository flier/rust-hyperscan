# rust-hyperscan [![Continuous integration](https://github.com/flier/rust-hyperscan/actions/workflows/ci.yml/badge.svg)](https://github.com/flier/rust-hyperscan/actions/workflows/ci.yml) [![crate](https://img.shields.io/crates/v/hyperscan.svg)](https://crates.io/crates/hyperscan) [![docs](https://docs.rs/hyperscan/badge.svg)](https://docs.rs/crate/hyperscan/)

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
features = ["full"]
```

### Chimera API

In order to improve regular expression compatibility, Hyperscan v5.0 starts to provide a PCRE-compatible [Chimera](http://intel.github.io/hyperscan/dev-reference/chimera.html) library.

To enable `Chimera` support, you need to manually download PCRE 8.41 or above, unzip to the source directory of Hyperscan 5.x, compile and install it.

```bash
$ cd hyperscan-5.3.0
$ wget https://ftp.pcre.org/pub/pcre/pcre-8.44.tar.gz
$ mkdir pcre
$ tar xvf pcre-8.44.tar.gz --strip-components=1 --directory pcre

$ mkdir build && cd build
$ cmake .. -DCMAKE_INSTALL_PREFIX=`pwd` -G Ninja
$ ninja
$ ninja install
```

Then point to the hyperscan installation directory with the `HYPERSCAN_ROOT` environment variable to enable `chimera` feature.

```bash
$ HYPERSCAN_ROOT=<CMAKE_INSTALL_PREFIX> cargo build
```

The `chimera` feature should be enabled.

```toml
[dependencies]
hyperscan = { version = "0.2", features = ["chimera"] }
```

Note: The `Chimera` library does not support dynamic library linking mode, `static` feature is automatically enabled when `chimera` is enabled.

### Static Linking Mode

As of version 0.2, `rust-hyperscan` uses dynamic library linking mode by default. If you need link a static library, you can use the `static` feature.

```toml
[dependencies]
hyperscan = { version = "0.2", features = ["static"] }
```

### Hyperscan Runtime

Hyperscan provides [a standalone runtime library](http://intel.github.io/hyperscan/dev-reference/serialization.html#the-runtime-library), which can be used separately. If you don't need to compile regular expressions at runtime, you can reduce the size of the executable using `runtime` mode and get rid of C++ dependencies.

```toml
[dependencies.hyperscan]
version = "0.2"
default-features = false
features = ["runtime"]
```
