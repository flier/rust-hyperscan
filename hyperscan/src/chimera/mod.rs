//! Chimera is a software regular expression matching engine that is a hybrid of Hyperscan and PCRE.
//!
//! The design goals of Chimera are to fully support PCRE syntax as well as to
//! take advantage of the high performance nature of Hyperscan.
//!
//! # Examples
//!
//! ```rust
//! # use hyperscan::chimera::prelude::*;
//! let db: Database = "/test/i".parse().unwrap();
//! let scratch = db.alloc_scratch().unwrap();
//! let mut matches = vec![];
//! let mut errors = vec![];
//!
//! db.scan("some test data", &scratch, |id, from, to, _flags, captured| {
//!     matches.push((from, to));
//!
//!     Matching::Continue
//! }, |error_type, id| {
//!     errors.push((error_type, id));
//!
//!     Matching::Skip
//! }).unwrap();
//!
//! assert_eq!(matches, vec![(5, 9)]);
//! assert_eq!(errors, vec![]);
//! ```
mod common;
mod compile;
mod error;
mod pattern;
mod runtime;

#[doc(hidden)]
pub use crate::ffi::chimera as ffi;

pub use self::common::{version, Database, DatabaseRef};
pub use self::compile::{compile, Builder, CompileError, Mode};
pub use self::error::Error;
pub use self::pattern::{Flags, Pattern, Patterns};
pub use self::runtime::{
    Capture, Error as MatchError, ErrorEventHandler, MatchEventHandler, Matching, Scratch, ScratchRef,
};

pub mod prelude {
    //! The `chimera` Prelude
    pub use crate::chimera::{
        compile, Builder, Capture, Database, DatabaseRef, Error, Matching, Pattern, Patterns, Scratch, ScratchRef,
    };
}
