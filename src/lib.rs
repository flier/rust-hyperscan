extern crate log;
extern crate libc;

mod raw;
mod constants;
mod cptr;
#[macro_use]
mod errors;
mod common;
#[macro_use]
mod compile;
mod runtime;

pub use common::{Type, Database, RawDatabase, BlockDatabase, StreamingDatabase, VectoredDatabase,
                 SerializedDatabase};
pub use constants::*;
pub use errors::Error;
pub use compile::{DatabaseBuilder, Expression, ExpressionInfo, Pattern};
pub use runtime::{Scratch, RawScratch};

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
extern crate regex;

#[cfg(test)]
mod tests {
    pub use common::tests::*;
}
