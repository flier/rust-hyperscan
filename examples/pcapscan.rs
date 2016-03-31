// Hyperscan example program 2: pcapscan
//
// This example is a very simple packet scanning benchmark. It scans a given
// PCAP file full of network traffic against a group of regular expressions and
// returns some coarse performance measurements.  This example provides a quick
// way to examine the performance achievable on a particular combination of
// platform, pattern set and input data.
//
// Build instructions:
//
//     cargo run --example pcapscan
//
// Usage:
//
//     ./pcapscan [-n repeats] <pattern file> <pcap file>
//
// We recommend the use of a utility like 'taskset' on multiprocessor hosts to
// pin execution to a single processor: this will remove processor migration
// by the scheduler as a source of noise in the results.
//
//
extern crate getopts;
extern crate pcap;

#[macro_use]
extern crate hyperscan;

use std::fmt;
use std::env;
use std::slice;
use std::error;
use std::process::exit;
use std::path::Path;
use std::num;
use std::io;
use std::io::{Write, BufRead};
use std::fs::File;
use std::iter::Iterator;

use getopts::Options;
use hyperscan::{CompileFlags, Pattern, Patterns, StreamingDatabase, BlockDatabase, DatabaseBuilder};

#[derive(Debug)]
enum Error {
    Io(io::Error),
    Parse(num::ParseIntError),
    Compile(hyperscan::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", error::Error::description(self))
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref err) => err.description(),
            Error::Parse(ref err) => err.description(),
            Error::Compile(ref err) => err.description(),
        }
    }
}

/**
 * This function will read in the file with the specified name, with an
 * expression per line, ignoring lines starting with '#' and build a Hyperscan
 * database for it.
 */
fn databases_from_file(filename: &str) -> Result<(StreamingDatabase, BlockDatabase), Error> {
    // do the actual file reading and string handling
    let patterns = try!(parse_file(filename).map_err(Error::Io));

    println!("Compiling Hyperscan databases with {} patterns.",
             patterns.len());

    Ok((try!(patterns.build().map_err(Error::Compile)),
        try!(patterns.build().map_err(Error::Compile))))
}

fn parse_file(filename: &str) -> Result<Patterns, io::Error> {
    let f = try!(File::open(filename));
    let lines = io::BufReader::new(f).lines();
    let patterns = lines.filter_map(|line: Result<String, io::Error>| -> Option<Pattern> {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                write!(io::stderr(), "ERROR: Could not read line, {}\n", err);
                exit(-1);
            }
        };

        let line = line.trim();

        if line.len() == 0 || line.starts_with('#') {
            None
        } else {
            match line.find(':') {
                Some(off) => unsafe {
                    if let Ok(id) = line.slice_unchecked(0, off).parse() {
                        Some(Pattern {
                            expression: String::from(line.slice_unchecked(off + 1, line.len())),
                            flags: CompileFlags(0),
                            id: id,
                        })
                    } else {
                        None
                    }
                },
                None => {
                    write!(io::stderr(), "ERROR: Could not parse line: {}\n", line);
                    exit(-1);
                }
            }
        }
    });

    Ok(patterns.collect())
}

struct Benchmark {
    db_streaming: StreamingDatabase,
    db_block: BlockDatabase,
    packets: Vec<[u8]>,
}

impl Benchmark {
    fn new(db_streaming: StreamingDatabase, db_block: BlockDatabase) -> Benchmark {
        Benchmark {
            db_streaming: db_streaming,
            db_block: db_block,
            packets: Vec::new(),
        }
    }

    fn read_streams(&self, pcap_file: &str) -> Result<(), pcap::Error> {
        let mut capture = try!(pcap::Capture::from_file(Path::new(pcap_file)));

        while let Ok(packet) = capture.next() {

        }

        Ok(())
    }

    // Display some information about the compiled database and scanned data.
    fn display_stat(&self) {}
}

// Main entry point.
fn main() {
    // Process command line arguments.
    let args: Vec<String> = env::args().collect();
    let prog = Path::new(&args[0]).file_name().unwrap().to_str().unwrap();
    let mut opts = Options::new();

    opts.optopt("n", "", "repeat times", "repeats");

    let usage = || {
        let brief = format!("Usage: {} [options] <pattern file> <pcap file>", prog);

        print!("{}", opts.usage(&brief));
    };

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(_) => {
            usage();
            exit(-1);
        }
    };

    let repeat_count: usize = match matches.opt_str("n") {
        Some(s) => {
            match s.parse() {
                Ok(n) => n,
                Err(err) => {
                    write!(io::stderr(),
                           "ERROR: Unable to parse repeats `{}`: {}\n",
                           s,
                           err);
                    exit(-1);
                }
            }
        }
        None => 1,
    };

    if matches.free.len() != 2 {
        usage();
        exit(-1);
    }

    let pattern_file = matches.free[0].as_str();
    let pcap_file = matches.free[1].as_str();

    // Read our pattern set in and build Hyperscan databases from it.
    println!("Pattern file: {}", pattern_file);

    let (db_streaming, db_block) = match databases_from_file(pattern_file) {
        Ok((db_streaming, db_block)) => (db_streaming, db_block),
        Err(err) => {
            write!(io::stderr(),
                   "ERROR: Unable to parse and compile patterns: {}\n",
                   err);
            exit(-1);
        }
    };

    // Read our input PCAP file in
    let bench = Benchmark::new(db_streaming, db_block);

    println!("PCAP input file: {}", pcap_file);

    if let Err(err) = bench.read_streams(pcap_file) {
        write!(io::stderr(),
               "Unable to read packets from PCAP file. Exiting. {}\n",
               err);
        exit(-1);
    }


    if repeat_count != 1 {
        println!("Repeating PCAP scan {} times.", repeat_count);
    }

    bench.displayStats();
}
