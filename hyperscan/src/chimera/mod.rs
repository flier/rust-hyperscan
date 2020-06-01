//! Chimera is a software regular expression matching engine that is a hybrid of Hyperscan and PCRE.
//!
//! The design goals of Chimera are to fully support PCRE syntax as well as to
//! take advantage of the high performance nature of Hyperscan.
mod common;
mod compile;
mod errors;
mod runtime;

pub use crate::ffi::chimera as ffi;

pub use self::common::{version, Database, DatabaseRef};
pub use self::compile::{Builder, Flags, Mode};
pub use self::errors::{CompileError, Error};
pub use self::runtime::{Scratch, ScratchRef};
