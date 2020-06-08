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

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use structopt::StructOpt;

use hyperscan::prelude::*;

#[derive(Debug, StructOpt)]
#[structopt(name = "simplegrep", about = "An example search a given input file for a pattern.")]
struct Opt {
    /// Regex pattern
    pattern: String,

    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    // First, we attempt to compile the pattern provided on the command line.
    // We assume 'DOTALL' semantics, meaning that the '.' meta-character will
    // match newline characters. The compiler will analyse the given pattern and
    // either return a compiled Hyperscan database, or an error message
    // explaining why the pattern didn't compile.
    //
    let pattern = Pattern::with_flags(
        opt.pattern,
        CompileFlags::DOTALL | CompileFlags::MULTILINE | CompileFlags::SOM_LEFTMOST,
    )
    .with_context(|| "parse pattern")?;

    let database: BlockDatabase = pattern.build().with_context(|| "compile pattern")?;

    // Next, we read the input data file into a buffer.
    let input_data = fs::read_to_string(opt.input).with_context(|| "read input file")?;

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

    let scratch = database.alloc_scratch().with_context(|| "allocate scratch space")?;

    println!("Scanning {} bytes with Hyperscan", input_data.len());

    database
        .scan(&input_data, &scratch, |_, from, to, _| {
            println!(
                "Match for pattern \"{}\" at offset {}..{}: {}",
                pattern.expression,
                from,
                to,
                &input_data[from as usize..to as usize]
            );

            Matching::Continue
        })
        .with_context(|| "scan input buffer")
}
