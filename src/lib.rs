extern crate libc;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
extern crate regex;

mod raw;
mod cptr;
#[macro_use]
mod common;
#[macro_use]
mod compile;
mod runtime;

pub use common::{Error, BlockDatabase, StreamingDatabase, VectoredDatabase, SerializedDatabase};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
