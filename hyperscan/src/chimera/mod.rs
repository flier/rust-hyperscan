//! Chimera is a software regular expression matching engine that is a hybrid of Hyperscan and PCRE.
//!
//! The design goals of Chimera are to fully support PCRE syntax as well as to
//! take advantage of the high performance nature of Hyperscan.
//!
//! # Example
//!
//! ```rust
//! # use hyperscan::chimera::prelude::*;
//! let pattern = pattern! {"test"; CASELESS};
//! let db = pattern.build().unwrap();
//! let scratch = db.alloc_scratch().unwrap();
//! let mut matches = vec![];
//! let mut errors = vec![];
//!
//! db.scan("some test data", &scratch, |id, from, to, _flags, captured| {
//!     println!("found pattern {} : {} @ [{}, {})", id, pattern.expression, from, to);
//!
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
mod errors;
mod runtime;

#[doc(hidden)]
pub use crate::ffi::chimera as ffi;

pub use self::common::{version, Database, DatabaseRef};
pub use self::compile::{Builder, Flags, Mode};
pub use self::errors::{CompileError, Error};
pub use self::runtime::{Matching, Scratch, ScratchRef};

pub mod prelude {
    //! The `chimera` Prelude
    pub use crate::chimera::{Builder, Database, DatabaseRef, Matching, Scratch, ScratchRef};
    pub use crate::{pattern, patterns, Pattern, Patterns};
}
