//! Chimera is a software regular expression matching engine that is a hybrid of Hyperscan and PCRE.
//!
//! The design goals of Chimera are to fully support PCRE syntax as well as to
//! take advantage of the high performance nature of Hyperscan.
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

/// The `chimera` Prelude
pub mod prelude {
    pub use crate::chimera::{Builder, Database, DatabaseRef, Matching, Scratch, ScratchRef};
    pub use crate::{pattern, patterns, Pattern, Patterns};
}
