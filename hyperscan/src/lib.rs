//! Hyperscan is a high-performance regular expression matching library.
//!
//! # Examples
//!
//! ```
//! #[macro_use]
//! extern crate hyperscan;
//!
//! use hyperscan::*;
//!
//! fn callback(id: u32, from: u64, to: u64, flags: u32, _: &BlockDatabase) -> u32 {
//!     assert_eq!(id, 0);
//!     assert_eq!(from, 5);
//!     assert_eq!(to, 9);
//!     assert_eq!(flags, 0);
//!
//!     println!("found pattern #{} @ [{}, {})", id, from, to);
//!
//!     0
//! }
//!
//! fn main() {
//!     let pattern = &pattern!{"test", flags => CompileFlags::CASELESS | CompileFlags::SOM_LEFTMOST};
//!     let db: BlockDatabase = pattern.build().unwrap();
//!     let scratch = db.alloc().unwrap();
//!
//!     db.scan("some test data", &scratch, Some(callback), Some(&db)).unwrap();
//! }
//! ```
#![deny(warnings, missing_docs, rust_2018_compatibility, rust_2018_idioms)]

#[macro_use]
extern crate log;

mod ffi {
    pub use hyperscan_sys::*;
}

mod constants;
#[macro_use]
mod errors;
mod common;
#[macro_use]
mod compile;
mod runtime;

pub use crate::common::{
    Block, BlockDatabase, CBuffer, Database, DatabaseRef, Mode, Serialized, Streaming, StreamingDatabase, Vectored,
    VectoredDatabase,
};
pub use crate::compile::{
    Builder, Error as CompileError, ExpressionExt, ExpressionInfo, Flags as CompileFlags, Pattern, Patterns,
    PlatformInfo, PlatformInfoRef,
};
pub use crate::constants::*;
pub use crate::errors::HsError;
pub use crate::runtime::{Scannable, Scratch, ScratchRef, Stream, StreamRef};

#[cfg(test)]
mod tests {
    pub use super::common::tests::*;
}
