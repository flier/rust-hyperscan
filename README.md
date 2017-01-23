# rust-hyperscan [![travis](https://api.travis-ci.org/flier/rust-hyperscan.svg)](https://travis-ci.org/flier/rust-hyperscan) [![crate](https://img.shields.io/crates/v/hyperscan.svg)](https://crates.io/crates/hyperscan) [![docs](https://docs.rs/hyperscan/badge.svg)](https://docs.rs/hyperscan)

[Hyperscan](https://github.com/01org/hyperscan) is a high-performance regular expression matching library.

## Usage

To use, add the following line to Cargo.toml under [dependencies]:

```toml
hyperscan = "0.1"
```
or alternatively,
```
hyperscan = { git = "https://github.com/flier/rust-hyperscan.git" }
```

## Example

```rust
#[macro_use]
extern crate hyperscan;

use hyperscan::*;

fn callback(id: u32, from: u64, to: u64, flags: u32, _: &BlockDatabase) -> u32 {
    assert_eq!(id, 0);
    assert_eq!(from, 5);
    assert_eq!(to, 9);
    assert_eq!(flags, 0);

    println!("found pattern #{} @ [{}, {})", id, from, to);

    0
}

fn main() {
    let pattern = &pattern!{"test", flags => HS_FLAG_CASELESS|HS_FLAG_SOM_LEFTMOST};
    let db: BlockDatabase = pattern.build().unwrap();
    let scratch = db.alloc().unwrap();

    db.scan::<BlockDatabase>("some test data", 0, &scratch, Some(callback), Some(&db)).unwrap();
}
```
