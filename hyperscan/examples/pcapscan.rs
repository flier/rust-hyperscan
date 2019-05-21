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
extern crate byteorder;
extern crate env_logger;
extern crate getopts;
extern crate hyperscan;
extern crate log;
extern crate pcap;
extern crate pnet;

use std::collections::HashMap;
use std::env;
use std::error;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::{BufRead, Write};
use std::iter::Iterator;
use std::net::SocketAddrV4;
use std::path::Path;
use std::process::exit;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use byteorder::{BigEndian, ReadBytesExt};
use getopts::Options;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::udp::UdpPacket;
use pnet::packet::{Packet, PrimitiveValues};

use hyperscan::{
    BlockDatabase, BlockScanner, Database, DatabaseBuilder, Pattern, Patterns, RawScratch, RawStream, Scratch,
    ScratchAllocator, Stream, StreamingDatabase, StreamingScanner,
};

#[derive(Debug)]
enum Error {
    IoError(io::Error),
    CompileError(hyperscan::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IoError(err)
    }
}

impl From<hyperscan::Error> for Error {
    fn from(err: hyperscan::Error) -> Error {
        Error::CompileError(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", error::Error::description(self))
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::IoError(ref err) => err.description(),
            Error::CompileError(ref err) => err.description(),
        }
    }
}

const NANOS_PER_MILLI: u32 = 1_000_000;
const MILLIS_PER_SEC: u64 = 1_000;

trait Milliseconds {
    fn ms(&self) -> usize;
}

impl Milliseconds for Duration {
    fn ms(&self) -> usize {
        (self.as_secs() * MILLIS_PER_SEC) as usize + (self.subsec_nanos() / NANOS_PER_MILLI) as usize
    }
}

macro_rules! build_database {
    ($builder:expr, $mode:expr) => {{
        let now = Instant::now();

        let db = try!($builder.build());

        println!("Hyperscan {} mode database compiled in {}ms", $mode, now.elapsed().ms());

        db
    }};
}

/**
 * This function will read in the file with the specified name, with an
 * expression per line, ignoring lines starting with '#' and build a Hyperscan
 * database for it.
 */
fn databases_from_file(filename: &str) -> Result<(StreamingDatabase, BlockDatabase), Error> {
    // do the actual file reading and string handling
    let patterns = try!(parse_file(filename));

    println!("Compiling Hyperscan databases with {} patterns.", patterns.len());

    Ok((
        build_database!(patterns, "streaming"),
        build_database!(patterns, "block"),
    ))
}

fn parse_file(filename: &str) -> Result<Patterns, io::Error> {
    let f = try!(File::open(filename));
    let patterns = io::BufReader::new(f)
        .lines()
        .filter_map(|line: Result<String, io::Error>| -> Option<Pattern> {
            if let Ok(line) = line {
                let line = line.trim();

                if line.len() > 0 && !line.starts_with('#') {
                    if let Ok(pattern) = Pattern::parse(line) {
                        return Some(pattern);
                    }
                }
            }

            None
        });

    Ok(patterns.collect())
}

// Key for identifying a stream in our pcap input data, using data from its IP
// headers.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct FiveTuple {
    proto: u8,
    src: SocketAddrV4,
    dst: SocketAddrV4,
}

impl FiveTuple {
    fn new(ipv4: &Ipv4Packet) -> FiveTuple {
        let mut c = io::Cursor::new(ipv4.payload());
        let src_port = c.read_u16::<BigEndian>().unwrap();
        let dst_port = c.read_u16::<BigEndian>().unwrap();

        FiveTuple {
            proto: ipv4.get_next_level_protocol().to_primitive_values().0,
            src: SocketAddrV4::new(ipv4.get_source(), src_port),
            dst: SocketAddrV4::new(ipv4.get_destination(), dst_port),
        }
    }
}

const IP_FLAG_MF: u8 = 1;

struct Benchmark {
    /// Packet data to be scanned.
    packets: Vec<Box<Vec<u8>>>,

    /// The stream ID to which each packet belongs
    stream_ids: Vec<usize>,

    /// Map used to construct stream_ids
    stream_map: HashMap<FiveTuple, usize>,

    /// Hyperscan compiled database (streaming mode)
    db_streaming: StreamingDatabase,

    /// Hyperscan compiled database (block mode)
    db_block: BlockDatabase,

    /// Hyperscan temporary scratch space (used in both modes)
    scratch: RawScratch,

    // Vector of Hyperscan stream state (used in streaming mode)
    streams: Vec<RawStream>,

    // Count of matches found during scanning
    match_count: AtomicUsize,
}

impl Benchmark {
    fn new(db_streaming: StreamingDatabase, db_block: BlockDatabase) -> Result<Benchmark, hyperscan::Error> {
        let mut s = try!(db_streaming.alloc());

        try!(s.realloc(&db_block));

        Ok(Benchmark {
            packets: Vec::new(),
            stream_ids: Vec::new(),
            stream_map: HashMap::new(),
            db_streaming: db_streaming,
            db_block: db_block,
            scratch: s,
            streams: Vec::new(),
            match_count: AtomicUsize::new(0),
        })
    }

    fn decode_packet(packet: &pcap::Packet) -> Option<(FiveTuple, Vec<u8>)> {
        let ether = EthernetPacket::new(&packet.data).unwrap();

        if ether.get_ethertype() != EtherTypes::Ipv4 {
            return None;
        }

        let ipv4 = Ipv4Packet::new(&ether.payload()).unwrap();

        if ipv4.get_version() != 4 {
            return None;
        }

        if (ipv4.get_flags() & IP_FLAG_MF) == IP_FLAG_MF || ipv4.get_fragment_offset() != 0 {
            return None;
        }

        match ipv4.get_next_level_protocol() {
            IpNextHeaderProtocols::Tcp => {
                let payload = ipv4.payload();
                let data_off = ((payload[12] >> 4) * 4) as usize;

                Some((FiveTuple::new(&ipv4), Vec::from(&payload[data_off..])))
            }

            IpNextHeaderProtocols::Udp => {
                let udp = UdpPacket::new(&ipv4.payload()).unwrap();

                Some((FiveTuple::new(&ipv4), Vec::from(udp.payload())))
            }
            _ => None,
        }
    }

    fn read_streams(&mut self, pcap_file: &str) -> Result<(), pcap::Error> {
        let mut capture = try!(pcap::Capture::from_file(Path::new(pcap_file)));

        while let Ok(ref packet) = capture.next() {
            if let Some((key, payload)) = Self::decode_packet(&packet) {
                if payload.len() > 0 {
                    let stream_id = match self.stream_map.get(&key) {
                        Some(&id) => id,
                        None => {
                            let id = self.stream_map.len();

                            assert!(self.stream_map.insert(key, id).is_none());

                            id
                        }
                    };

                    self.stream_ids.push(stream_id);
                    self.packets.push(Box::new(payload));
                }
            }
        }

        Ok(())
    }

    // Return the number of bytes scanned
    fn bytes(&self) -> usize {
        self.packets.iter().fold(0, |bytes, p| bytes + p.len())
    }

    // Return the number of matches found.
    fn matches(&self) -> usize {
        self.match_count.load(Ordering::Relaxed)
    }

    // Clear the number of matches found.
    fn clear_matches(&mut self) {
        self.match_count.store(0, Ordering::Relaxed);
    }

    fn on_match(_: u32, _: u64, _: u64, _: u32, match_count: &AtomicUsize) -> u32 {
        match_count.fetch_add(1, Ordering::Relaxed);

        0
    }

    // Open a Hyperscan stream for each stream in stream_ids
    fn open_streams(&mut self) {
        self.streams = self
            .stream_map
            .iter()
            .map(|_| self.db_streaming.open_stream(0).unwrap())
            .collect()
    }

    // Close all open Hyperscan streams (potentially generating any end-anchored matches)
    fn close_streams(&mut self) {
        for ref stream in &self.streams {
            if let Err(err) = stream.close(&self.scratch, Some(Self::on_match), Some(&self.match_count)) {
                println!("ERROR: Unable to close stream. Exiting. {}", err);
            }
        }
    }

    fn reset_streams(&mut self) {
        for ref stream in &self.streams {
            if let Err(err) = stream.reset(0, &self.scratch, Some(Self::on_match), Some(&self.match_count)) {
                println!("ERROR: Unable to reset stream. Exiting. {}", err);
            }
        }
    }

    // Scan each packet (in the ordering given in the PCAP file)
    // through Hyperscan using the streaming interface.
    fn scan_streams(&mut self) {
        for (i, ref packet) in self.packets.iter().enumerate() {
            let ref stream = self.streams[self.stream_ids[i]];

            if let Err(err) = stream.scan(
                packet.as_ref().as_slice(),
                0,
                &self.scratch,
                Some(Self::on_match),
                Some(&self.match_count),
            ) {
                println!("ERROR: Unable to scan packet. Exiting. {}", err)
            }
        }
    }

    // Scan each packet (in the ordering given in the PCAP file)
    // through Hyperscan using the block-mode interface.
    fn scan_block(&mut self) {
        for ref packet in &self.packets {
            if let Err(err) = self.db_block.scan(
                packet.as_ref().as_slice(),
                0,
                &self.scratch,
                Some(Self::on_match),
                Some(&self.match_count),
            ) {
                println!("ERROR: Unable to scan packet. Exiting. {}", err)
            }
        }
    }

    // Display some information about the compiled database and scanned data.
    fn display_stats(&self) {
        let num_packets = self.packets.len();
        let num_streams = self.stream_map.len();
        let num_bytes = self.bytes();

        println!(
            "{} packets in {} streams, totalling {} bytes.",
            num_packets, num_streams, num_bytes
        );
        println!(
            "Average packet length: {} bytes.",
            num_bytes / if num_packets > 0 { num_packets } else { 1 }
        );
        println!(
            "Average stream length: {} bytes.",
            num_bytes / if num_streams > 0 { num_streams } else { 1 }
        );
        println!("");

        match self.db_streaming.database_size() {
            Ok(size) => {
                println!("Streaming mode Hyperscan database size    : {} bytes.", size);
            }
            Err(err) => println!("Error getting streaming mode Hyperscan database size, {}", err),
        }

        match self.db_block.database_size() {
            Ok(size) => {
                println!("Block mode Hyperscan database size        : {} bytes.", size);
            }
            Err(err) => println!("Error getting block mode Hyperscan database size, {}", err),
        }

        match self.db_streaming.stream_size() {
            Ok(size) => {
                println!(
                    "Streaming mode Hyperscan stream state size: {} bytes (per stream).",
                    size
                );
            }
            Err(err) => println!("Error getting stream state size, {}", err),
        }
    }
}

// Main entry point.
#[allow(unused_must_use)]
fn main() {
    env_logger::init();

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
        Some(s) => match s.parse() {
            Ok(n) => n,
            Err(err) => {
                write!(io::stderr(), "ERROR: Unable to parse repeats `{}`: {}\n", s, err);
                exit(-1);
            }
        },
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
            write!(io::stderr(), "ERROR: Unable to parse and compile patterns: {}\n", err);
            exit(-1);
        }
    };

    // Read our input PCAP file in
    let mut bench = Benchmark::new(db_streaming, db_block).unwrap();

    println!("PCAP input file: {}", pcap_file);

    if let Err(err) = bench.read_streams(pcap_file) {
        write!(
            io::stderr(),
            "Unable to read packets from PCAP file. Exiting. {}\n",
            err
        );
        exit(-1);
    }

    if repeat_count != 1 {
        println!("Repeating PCAP scan {} times.", repeat_count);
    }

    bench.display_stats();

    // Streaming mode scans.
    let mut streaming_scan = Duration::from_secs(0);
    let mut streaming_open_close = Duration::from_secs(0);

    for i in 0..repeat_count {
        if i == 0 {
            // Open streams.
            let now = Instant::now();
            bench.open_streams();
            streaming_open_close = streaming_open_close + now.elapsed();
        } else {
            // Reset streams.
            let now = Instant::now();
            bench.reset_streams();
            streaming_open_close = streaming_open_close + now.elapsed();
        }

        // Scan all our packets in streaming mode.
        let now = Instant::now();
        bench.scan_streams();
        streaming_scan = streaming_scan + now.elapsed();
    }

    // Close streams.
    let now = Instant::now();
    bench.close_streams();
    streaming_open_close = streaming_open_close + now.elapsed();

    // Collect data from streaming mode scans.
    let bytes = bench.bytes();
    let total_bytes = (bytes * 8 * repeat_count) as f64;
    let tput_stream_scanning = total_bytes * 1000.0 / streaming_scan.ms() as f64;
    let tput_stream_overhead = total_bytes * 1000.0 / (streaming_scan + streaming_open_close).ms() as f64;
    let matches_stream = bench.matches();
    let match_rate_stream = (matches_stream as f64) / ((bytes * repeat_count) as f64 / 1024.0);

    // Scan all our packets in block mode.
    bench.clear_matches();
    let now = Instant::now();
    for _ in 0..repeat_count {
        bench.scan_block();
    }
    let scan_block = now.elapsed();

    // Collect data from block mode scans.
    let tput_block_scanning = total_bytes * 1000.0 / scan_block.ms() as f64;
    let matches_block = bench.matches();
    let match_rate_block = (matches_block as f64) / ((bytes * repeat_count) as f64 / 1024.0);

    println!("\nStreaming mode:\n");
    println!("  Total matches: {}", matches_stream);
    println!("  Match rate:    {:.4} matches/kilobyte", match_rate_stream);
    println!(
        "  Throughput (with stream overhead): {:.2} megabits/sec",
        tput_stream_overhead / 1000000.0
    );
    println!(
        "  Throughput (no stream overhead):   {:.2} megabits/sec",
        tput_stream_scanning / 1000000.0
    );

    println!("\nBlock mode:\n");
    println!("  Total matches: {}", matches_block);
    println!("  Match rate:    {:.4} matches/kilobyte", match_rate_block);
    println!("  Throughput:    {:.2} megabits/sec", tput_block_scanning / 1000000.0);

    if bytes < (2 * 1024 * 1024) {
        println!(
            "\nWARNING: Input PCAP file is less than 2MB in size.\n
                  This test may have been too short to calculate accurate results."
        );
    }
}
