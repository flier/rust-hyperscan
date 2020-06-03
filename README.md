# rust-hyperscan [![travis](https://api.travis-ci.org/flier/rust-hyperscan.svg)](https://travis-ci.org/flier/rust-hyperscan) [![crate](https://img.shields.io/crates/v/hyperscan.svg)](https://crates.io/crates/hyperscan) [![docs](https://docs.rs/hyperscan/badge.svg)](https://docs.rs/crate/hyperscan/)

[Hyperscan](https://github.com/01org/hyperscan) is a high-performance regular expression matching library.

## Usage

To use, add the following line to Cargo.toml under [dependencies]:

```toml
hyperscan = "0.1"
```

## Examples

```rust
use hyperscan::prelude::*;

fn main() {
    let pattern = &pattern! {"test"; CASELESS | SOM_LEFTMOST};
    let db: BlockDatabase = pattern.build().unwrap();
    let scratch = db.alloc_scratch().unwrap();

    db.scan("some test data", &scratch, |id, from, to, flags| {
        assert_eq!(id, 0);
        assert_eq!(from, 5);
        assert_eq!(to, 9);
        assert_eq!(flags, 0);

        println!("found pattern #{} @ [{}, {})", id, from, to);

        Matching::Continue
    }).unwrap();
}
```
