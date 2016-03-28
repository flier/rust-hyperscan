extern crate log;
extern crate libc;
extern crate regex_syntax;

mod raw;
mod constants;
mod cptr;
#[macro_use]
mod errors;
mod common;
#[macro_use]
mod compile;
mod runtime;

pub use common::{Type, Block, Streaming, Vectored, SerializableDatabase, SerializedDatabase,
                 Database, RawDatabase, BlockDatabase, StreamingDatabase, VectoredDatabase};
pub use constants::*;
pub use errors::Error;
pub use compile::{CompileFlags, Expression, ExpressionInfo, Pattern, DatabaseBuilder};
pub use runtime::{Scratch, RawScratch, MatchEventCallback, BlockScanner, VectoredScanner,
                  StreamingScanner, StreamFlags, Stream, RawStream};

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
extern crate regex;

#[cfg(test)]
mod tests {
    pub use common::tests::*;
}
