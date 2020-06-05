mod closure;
#[cfg(feature = "pattern")]
mod pattern;
mod scan;
mod scratch;
mod stream;

pub use self::closure::split_closure;
pub use self::scan::{MatchEventHandler, Matching};
pub use self::scratch::{Scratch, ScratchRef};
pub use self::stream::{Stream, StreamRef};
