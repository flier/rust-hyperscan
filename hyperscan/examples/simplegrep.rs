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

use std::env;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;
use std::pin::Pin;
use std::process::exit;

use failure::{Error, ResultExt};

use hyperscan::*;

/**
 * Fill a data buffer from the given filename, returning it and filling @a
 * length with its length. Returns NULL on failure.
 */
fn read_input_data(input_filename: &str) -> Result<String, io::Error> {
    let mut f = File::open(input_filename)?;
    let mut buf = String::new();

    f.read_to_string(&mut buf)?;

    Ok(buf)
}

fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let mut args = env::args();

    if args.len() != 3 {
        eprintln!(
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
    let pattern = pattern! { args.next().unwrap(); DOTALL };
    let input_filename = args.next().unwrap();

    let database = pattern.build::<Block>().context("compile pattern")?;

    // Next, we read the input data file into a buffer.
    let input_data = read_input_data(&input_filename).context("read input file")?;

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

    let scratch = database.alloc().context("allocate scratch space")?;

    println!("Scanning {} bytes with Hyperscan", input_data.len());

    // This is the function that will be called for each match that occurs.
    fn event_handler<'a>(_: u32, _: u64, to: u64, _: u32, expression: Option<Pin<&'a String>>) -> u32 {
        println!("Match for pattern \"{}\" at offset {}", expression.unwrap(), to);

        0
    };

    let expr = pattern.expression;

    let _ = database
        .scan(&input_data, &scratch, Some(event_handler), Some(Pin::new(&expr)))
        .context("scan input buffer")?;

    Ok(())
}
