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

#[macro_use]
extern crate log;

extern crate hyperscan_sys as ffi;

mod constants;
#[macro_use]
mod errors;
mod common;
#[macro_use]
mod compile;
mod runtime;

pub use crate::common::{
    Block, BlockDatabase, Database, DatabaseRef, Mode, SerializedDatabase, Streaming, StreamingDatabase, Vectored,
    VectoredDatabase,
};
pub use crate::compile::{
    Builder, Error as CompileError, ExpressionExt, ExpressionInfo, Flags as CompileFlags, Pattern, Patterns,
    PlatformInfo, PlatformInfoRef,
};
pub use crate::constants::*;
pub use crate::errors::HsError;
pub use crate::runtime::{Scratch, ScratchRef, Stream, StreamRef};

#[cfg(test)]
extern crate regex;

#[cfg(test)]
mod tests {
    pub use super::common::tests::*;
}
