//! Hyperscan is a high-performance regular expression matching library.
//!
//! # Building a Database
//!
//! Hyperscan provides three different scanning modes.
//!
//! - **Streaming mode**: the target data to be scanned is a continuous stream, not all of which is available at once;
//! blocks of data are scanned in sequence and matches may span multiple blocks in a stream.
//! In streaming mode, each stream requires a block of memory to store its state between scan calls.
//! - **Block mode**: the target data is a discrete, contiguous block which can be scanned in one call
//! and does not require state to be retained.
//! - **Vectored mode**: the target data consists of a list of non-contiguous blocks that are available all at once.
//! As for block mode, no retention of state is required.
//!
//! # Streaming Mode
//!
//! The streaming runtime API consists of functions to open, scan, and close Hyperscan data streams –
//! these functions being `StreamingScanner::open_stream()`, `<RawStream as BlockScanner>::scan()`,
//! and `Stream::close()`.
//! Any matches detected in the written data are returned to the calling application via a function pointer callback.
//!
//! ## Examples
//!
//! ```
//! #[macro_use]
//! extern crate hyperscan;
//!
//! use std::cell::RefCell;
//!
//! use hyperscan::*;
//!
//! extern "C" fn callback(_id: u32, from: u64, to: u64, _flags: u32, matches: &RefCell<Vec<(u64, u64)>>) -> u32 {
//!     (*matches.borrow_mut()).push((from, to));
//!
//!     0 // 0 - continue, 1 - terminate
//! }
//!
//! fn main() {
//!     // If SOM was requested for the pattern (see Start of Match),
//!     // the from argument will be set to the leftmost possible start-offset for the match.
//!     let pattern = &pattern!{"test", flags => CompileFlags::HS_FLAG_CASELESS | CompileFlags::HS_FLAG_SOM_LEFTMOST};
//!     // Build streaming database
//!     let db: StreamingDatabase = pattern.build().unwrap();
//!     // Allocate scratch to store on-the-fly internal data.
//!     let mut scratch = db.alloc().unwrap();
//!     // Open stream to scan data
//!     let stream = db.open_stream(0).unwrap();
//!     // Collect matched location (from, to)
//!     let matches = RefCell::new(Vec::new());
//!
//!     stream.scan("some te", 0, &mut scratch, Some(callback), Some(&matches)).unwrap();
//!     stream.scan("st data", 0, &mut scratch, Some(callback), Some(&matches)).unwrap();
//!     stream.close(&mut scratch, Some(callback), Some(&matches)).unwrap();
//!
//!     assert_eq!(matches.into_inner(), vec![(5, 9)]);
//! }
//! ```
//!
//! # Block Mode
//!
//! The block mode runtime API consists of a single method: `BlockScanner::scan()`.
//! Using the compiled patterns this function identifies matches in the target data,
//! using a function pointer callback to communicate with the application.
//!
//! ## Examples
//!
//! ```
//! #[macro_use]
//! extern crate hyperscan;
//!
//! use std::cell::RefCell;
//!
//! use hyperscan::*;
//!
//! extern "C" fn callback(_id: u32, from: u64, to: u64, _flags: u32, matches: &RefCell<Vec<(u64, u64)>>) -> u32 {
//!     (*matches.borrow_mut()).push((from, to));
//!
//!     0 // 0 - continue, 1 - terminate
//! }
//!
//! fn main() {
//!     // If SOM was requested for the pattern (see Start of Match),
//!     // the from argument will be set to the leftmost possible start-offset for the match.
//!     let pattern = &pattern!{"test", flags => CompileFlags::HS_FLAG_CASELESS | CompileFlags::HS_FLAG_SOM_LEFTMOST};
//!     // Build block database
//!     let db: BlockDatabase = pattern.build().unwrap();
//!     // Allocate scratch to store on-the-fly internal data.
//!     let mut scratch = db.alloc().unwrap();
//!     // Collect matched location (from, to)
//!     let matches = RefCell::new(Vec::new());
//!
//!     db.scan("some test data", 0, &mut scratch, Some(callback), Some(&matches)).unwrap();
//!
//!     assert_eq!(matches.into_inner(), vec![(5, 9)]);
//! }
//! ```
//!
//! # Vectored Mode
//!
//! The vectored mode runtime API, like the block mode API, consists of a single function: `VectoredScanner::scan()`.
//! This function accepts an array of data pointers and lengths,
//! facilitating the scanning in sequence of a set of data blocks that are not contiguous in memory.
//!
//! From the caller’s perspective, this mode will produce the same matches as if the set of data blocks were
//! (a) scanned in sequence with a series of streaming mode scans,
//! or (b) copied in sequence into a single block of memory and then scanned in block mode.
//!
//! ## Examples
//!
//! ```
//! #[macro_use]
//! extern crate hyperscan;
//!
//! use std::cell::RefCell;
//!
//! use hyperscan::*;
//!
//! extern "C" fn callback(_id: u32, from: u64, to: u64, _flags: u32, matches: &RefCell<Vec<(u64, u64)>>) -> u32 {
//!     (*matches.borrow_mut()).push((from, to));
//!
//!     0 // 0 - continue, 1 - terminate
//! }
//!
//! fn main() {
//!     // If SOM was requested for the pattern (see Start of Match),
//!     // the from argument will be set to the leftmost possible start-offset for the match.
//!     let pattern = &pattern!{"test", flags => CompileFlags::HS_FLAG_CASELESS | CompileFlags::HS_FLAG_SOM_LEFTMOST};
//!     // Build block database
//!     let db: VectoredDatabase = pattern.build().unwrap();
//!     // Allocate scratch to store on-the-fly internal data.
//!     let mut scratch = db.alloc().unwrap();
//!     // Collect matched location (from, to)
//!     let matches = RefCell::new(Vec::new());
//!
//!     db.scan(&["some", "test", "data"], 0, &mut scratch, Some(callback), Some(&matches)).unwrap();
//!
//!     assert_eq!(matches.into_inner(), vec![(4, 8)]);
//! }
//! ```
//!
//! # Scratch Space
//!
//! While scanning data, Hyperscan needs a small amount of temporary memory to store on-the-fly internal data.
//! This amount is unfortunately too large to fit on the stack, particularly for embedded applications,
//! and allocating memory dynamically is too expensive,
//! so a pre-allocated “scratch” space must be provided to the scanning functions.
//!
//! The function `ScratchAllocator::alloc()` allocates a large enough region of scratch space
//! to support a given database. If the application uses multiple databases,
//! only a single scratch region is necessary: in this case, calling `Scratch::realloc()`
//! on each database (with the same scratch pointer) will ensure that the scratch space is large enough
//! to support scanning against any of the given databases.
//!
//! While the Hyperscan library is re-entrant, the use of scratch spaces is not.
//! For example, if by design it is deemed necessary to run recursive or nested scanning
//! (say, from the match callback function), then an additional scratch space is required for that context.
//!
//! In the absence of recursive scanning, only one such space is required per thread and can
//! (and indeed should) be allocated before data scanning is to commence.
//!

#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy(conf_file = ".clippy.toml")))]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate failure;
extern crate hexplay;
extern crate libc;
#[macro_use]
extern crate log;
extern crate regex_syntax;

mod constants;
mod raw;
#[macro_use]
mod errors;
mod api;
mod common;
#[macro_use]
mod compile;
pub mod regex;
mod runtime;

pub use api::*;
pub use common::{valid_platform, version, Block, BlockDatabase, RawDatabase, StreamMode, Streaming, StreamingDatabase,
                 Vectored, VectoredDatabase};
pub use compile::{DatabaseCompiler, Pattern, Patterns};
pub use constants::*;
pub use runtime::{RawScratch, RawStream};

#[cfg(test)]
extern crate regex as re;
