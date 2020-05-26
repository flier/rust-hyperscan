//! Hyperscan is a high-performance regular expression matching library.
//!
//! # Usage
//!
//! This crate is on crates.io and can be used by adding `hyperscan` to your dependencies in your project's Cargo.toml.
//!
//! ```toml
//! [dependencies]
//! hyperscan = "0.2"
//! ```
//!
//! # Examples
//!
//! ```
//! #[macro_use]
//! extern crate hyperscan;
//!
//! use hyperscan::prelude::*;
//!
//! fn main() {
//!     let pattern = pattern! {"test"; CASELESS | SOM_LEFTMOST};
//!     let db: BlockDatabase = pattern.build().unwrap();
//!     let scratch = db.alloc_scratch().unwrap();
//!
//!     db.scan("some test data", &scratch, |id, from, to, _flags| {
//!         assert_eq!(id, 0);
//!         assert_eq!(from, 5);
//!         assert_eq!(to, 9);
//!
//!         println!("found pattern {} : {} @ [{}, {})", id, pattern.expression, from, to);
//!
//!         Matching::Continue
//!     }).unwrap();
//! }
//! ```
#![deny(missing_docs, rust_2018_compatibility, rust_2018_idioms)]
#![cfg_attr(test, deny(warnings))]
#![cfg_attr(feature = "pattern", feature(pattern))]

#[macro_use]
extern crate log;

mod ffi {
    pub use hyperscan_sys::*;
}

mod common;
mod errors;
#[macro_use]
mod compile;
pub mod regex;
mod runtime;

#[doc(hidden)]
#[deprecated = "use `BlockMode` instead"]
pub use crate::common::Block;
#[doc(hidden)]
#[deprecated = "use `SerializedDatabase` instead"]
pub use crate::common::Serialized;
#[doc(hidden)]
#[deprecated = "use `StreamingMode` instead"]
pub use crate::common::Streaming;
#[doc(hidden)]
#[deprecated = "use `VectoredMode` instead"]
pub use crate::common::Vectored;

pub use crate::common::{
    Block as BlockMode, BlockDatabase, Database, DatabaseRef, Mode, Serialized as SerializedDatabase,
    Streaming as StreamingMode, StreamingDatabase, Vectored as VectoredMode, VectoredDatabase,
};
pub use crate::compile::{
    Builder as DatabaseBuilder, Builder, CpuFeatures, Error as CompileError, ExpressionExt, ExpressionInfo,
    Flags as CompileFlags, Pattern, Patterns, Platform, PlatformRef, SomHorizon, Tune,
};
pub use crate::errors::HsError;
pub use crate::runtime::{Matching, Scannable, Scratch, ScratchRef, Stream, StreamRef};

/// The `hyperscan` Prelude
pub mod prelude {
    pub use crate::{
        pattern, BlockDatabase, Builder, CompileFlags, Database, Matching, Mode, Pattern, Patterns, Scratch, Stream,
        StreamingDatabase, VectoredDatabase,
    };
}

#[cfg(doctest)]
#[macro_use]
extern crate doc_comment;

#[cfg(doctest)]
doctest!("../../README.md");

#[cfg(test)]
mod tests {
    pub use super::common::tests::*;
}
