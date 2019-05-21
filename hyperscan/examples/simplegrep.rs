// Hyperscan example program 1: simplegrep
//
// This is a simple example of Hyperscan's most basic functionality: it will
// search a given input file for a pattern supplied as a command-line argument.
// It is intended to demonstrate correct usage of the hs_compile and hs_scan
// functions of Hyperscan.
//
// Patterns are scanned in 'DOTALL' mode, which is equivalent to PCRE's '/s'
// modifier. This behaviour can be changed by modifying the "flags" argument to
// hs_compile.
//
// Build instructions:
//
//     cargo run --example simplegrep
//
// Usage:
//
//     ./simplegrep <pattern> <input file>
//
// Example:
//
//     ./simplegrep int simplegrep.c
//
//

extern crate hyperscan;

use std::env;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::Path;
use std::process::exit;

use hyperscan::*;

/**
 * Fill a data buffer from the given filename, returning it and filling @a
 * length with its length. Returns NULL on failure.
 */
fn read_input_data(input_filename: &str) -> Result<String, io::Error> {
    let mut f = try!(File::open(input_filename));
    let mut buf = String::new();

    try!(f.read_to_string(&mut buf));

    Ok(buf)
}

#[allow(unused_must_use)]
fn main() {
    let mut args = env::args();

    if args.len() != 3 {
        write!(
            io::stderr(),
            "Usage: {} <pattern> <input file>\n",
            Path::new(&args.next().unwrap()).file_name().unwrap().to_str().unwrap()
        );
        exit(-1);
    }

    // First, we attempt to compile the pattern provided on the command line.
    // We assume 'DOTALL' semantics, meaning that the '.' meta-character will
    // match newline characters. The compiler will analyse the given pattern and
    // either return a compiled Hyperscan database, or an error message
    // explaining why the pattern didn't compile.
    //
    let _ = args.next();
    let pattern = pattern!(args.next().unwrap(), flags => HS_FLAG_DOTALL);
    let input_filename = args.next().unwrap();

    let database: BlockDatabase = match pattern.build() {
        Ok(db) => db,
        Err(err) => {
            write!(
                io::stderr(),
                "ERROR: Unable to compile pattern `{}`: {}\n",
                pattern,
                err
            );
            exit(-1);
        }
    };

    // Next, we read the input data file into a buffer.
    let input_data = match read_input_data(&input_filename) {
        Ok(buf) => buf,
        Err(err) => {
            write!(
                io::stderr(),
                "ERROR: Unable to read file `{}`: {}\n",
                input_filename,
                err
            );
            exit(-1);
        }
    };

    // Finally, we issue a call to hs_scan, which will search the input buffer
    // for the pattern represented in the bytecode. Note that in order to do
    // this, scratch space needs to be allocated with the hs_alloc_scratch
    // function. In typical usage, you would reuse this scratch space for many
    // calls to hs_scan, but as we're only doing one, we'll be allocating it
    // and deallocating it as soon as our matching is done.
    //
    // When matches occur, the specified callback function (eventHandler in
    // this file) will be called. Note that although it is reminiscent of
    // asynchronous APIs, Hyperscan operates synchronously: all matches will be
    // found, and all callbacks issued, *before* hs_scan returns.
    //
    // In this example, we provide the input pattern as the context pointer so
    // that the callback is able to print out the pattern that matched on each
    // match event.
    //

    let scratch = match database.alloc() {
        Ok(s) => s,
        Err(err) => {
            write!(io::stderr(), "ERROR: Unable to allocate scratch space. {}\n", err);
            exit(-1);
        }
    };

    println!("Scanning {} bytes with Hyperscan", input_data.len());

    // This is the function that will be called for each match that occurs.
    fn event_handler(_: u32, _: u64, to: u64, _: u32, pattern: &hyperscan::Pattern) -> u32 {
        println!("Match for pattern \"{}\" at offset {}", &pattern, to);

        0
    };

    if let Err(err) = database.scan(input_data.as_str(), 0, &scratch, Some(event_handler), Some(&pattern)) {
        write!(io::stderr(), "ERROR: Unable to scan input buffer. Exiting. {}\n", err);
        exit(-1);
    }
}
