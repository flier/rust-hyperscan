extern crate log;
extern crate libc;

mod raw;
mod cptr;
#[macro_use]
mod common;
mod constants;
#[macro_use]
mod compile;
mod runtime;

pub use common::{Error, BlockDatabase, StreamingDatabase, VectoredDatabase, SerializedDatabase};
pub use constants::*;
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
