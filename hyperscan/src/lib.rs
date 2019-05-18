//! Hyperscan is a high-performance regular expression matching library.
//!
//! # Examples
//!
//! ```
//! #[macro_use]
//! extern crate hyperscan;
//!
//! use std::pin::Pin;
//!
//! use hyperscan::prelude::*;
//!
//! fn callback(id: u32, from: u64, to: u64, flags: u32, expr: Option<Pin<&String>>) -> u32 {
//!     assert_eq!(id, 0);
//!     assert_eq!(from, 5);
//!     assert_eq!(to, 9);
//!     assert_eq!(flags, 0);
//!
//!     println!("found pattern {} : {} @ [{}, {})", id, expr.unwrap(), from, to);
//!
//!     0
//! }
//!
//! fn main() {
//!     let pattern = &pattern! {"test"; CASELESS | SOM_LEFTMOST};
//!     let db: BlockDatabase = pattern.build().unwrap();
//!     let scratch = db.alloc().unwrap();
//!     let expr = Pin::new(&pattern.expression);
//!
//!     db.scan("some test data", &scratch, Some(callback), Some(expr)).unwrap();
//! }
//! ```
#![deny(missing_docs, rust_2018_compatibility, rust_2018_idioms)]
#![cfg_attr(unstable, feature(pattern))]

#[macro_use]
extern crate log;

mod ffi {
    pub use hyperscan_sys::*;
}

mod common;
mod errors;
#[macro_use]
mod compile;
mod runtime;

pub use crate::common::{
    Block, BlockDatabase, CBuffer, Database, DatabaseRef, Mode, Serialized, Streaming, StreamingDatabase, Vectored,
    VectoredDatabase,
};
pub use crate::compile::{
    Builder, CpuFeatures, Error as CompileError, ExpressionExt, ExpressionInfo, Flags as CompileFlags, Pattern,
    Patterns, Platform, PlatformRef, SomHorizon, Tune,
};
pub use crate::errors::HsError;
pub use crate::runtime::{Scannable, Scratch, ScratchRef, Stream, StreamRef};

/// The `hyperscan` Prelude
pub mod prelude {
    pub use crate::{
        pattern, BlockDatabase, Builder, CompileFlags, Database, Mode, Pattern, Patterns, Scratch, Stream,
        StreamingDatabase, VectoredDatabase,
    };
}

#[cfg(test)]
mod tests {
    pub use super::common::tests::*;
}
