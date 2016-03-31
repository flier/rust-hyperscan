extern crate log;
extern crate libc;
extern crate regex_syntax;

mod raw;
mod constants;
mod cptr;
#[macro_use]
mod errors;
mod api;
mod common;
#[macro_use]
mod compile;
mod runtime;

pub use constants::*;
pub use api::*;
pub use errors::Error;
pub use common::{RawDatabase, BlockDatabase, StreamingDatabase, VectoredDatabase};
pub use compile::{CompileFlags, Pattern, Patterns};
pub use runtime::{RawScratch, RawStream};

#[cfg(test)]
extern crate regex;

#[cfg(test)]
mod tests {
    pub use common::tests::*;
}
