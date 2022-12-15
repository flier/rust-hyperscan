//! Hyperscan is a high-performance regular expression matching library.
//!
//! # Usage
//!
//! This crate is on crates.io and can be used by adding `hyperscan` to your dependencies in your project's Cargo.toml.
//!
//! ```toml
//! [dependencies]
//! hyperscan = "0.3"
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

mod ffi {
    pub use hyperscan_sys::*;
}

mod common;
mod error;
#[cfg(feature = "compile")]
#[macro_use]
mod compile;
#[cfg(feature = "chimera")]
pub mod chimera;
#[cfg(all(feature = "compile", feature = "runtime"))]
pub mod regex;
#[cfg(feature = "runtime")]
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
    version, version_str, Block as BlockMode, BlockDatabase, Database, DatabaseRef, Error as HsError, Mode,
    Serialized as SerializedDatabase, Streaming as StreamingMode, StreamingDatabase, Vectored as VectoredMode,
    VectoredDatabase,
};
pub use crate::error::{Error, Result};

cfg_if::cfg_if! {
    if #[cfg(feature = "compile")] {
        #[doc(hidden)]
        #[deprecated = "use `ExprExt` instead"]
        pub use crate::compile::ExprExt as ExpressionExt;
        #[doc(hidden)]
        #[deprecated = "use `ExprInfo` instead"]
        pub use crate::compile::ExprInfo as ExpressionInfo;
        #[doc(hidden)]
        #[deprecated = "use `PatternFlags` instead"]
        pub use crate::compile::Flags as CompileFlags;
        pub use crate::compile::{
            compile, Builder as DatabaseBuilder, Builder, CpuFeatures, Error as CompileError, ExprExt, ExprInfo,
            Flags as PatternFlags, Pattern, Patterns, Platform, PlatformRef, SomHorizon, Tune,
        };
        #[cfg(feature = "literal")]
        pub use crate::compile::{Literal, LiteralFlags, Literals};
    }
}

#[cfg(feature = "runtime")]
pub use crate::runtime::{MatchEventHandler, Matching, Scratch, ScratchRef, Stream, StreamRef};

/// The `hyperscan` Prelude
pub mod prelude {
    #[cfg(feature = "compile")]
    pub use crate::{compile, pattern, Builder, CompileFlags, Pattern, Patterns};

    #[cfg(feature = "runtime")]
    pub use crate::{Matching, Scratch, Stream};

    pub use crate::{BlockDatabase, Database, Mode, StreamingDatabase, VectoredDatabase};
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
