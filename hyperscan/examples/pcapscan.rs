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
use std::collections::HashMap;
use std::fs;
use std::io;
use std::iter;
use std::net::SocketAddrV4;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use byteorder::{BigEndian, ReadBytesExt};
use pnet::packet::{
    ethernet::{EtherTypes, EthernetPacket},
    ip::IpNextHeaderProtocols,
    ipv4::Ipv4Packet,
    udp::UdpPacket,
    Packet, PrimitiveValues,
};
use structopt::StructOpt;

use hyperscan::prelude::*;

/**
 * This function will read in the file with the specified name, with an
 * expression per line, ignoring lines starting with '#' and build a Hyperscan
 * database for it.
 */
fn read_databases<P: AsRef<Path>>(path: P) -> Result<(StreamingDatabase, BlockDatabase)> {
    // do the actual file reading and string handling
    let patterns: Patterns = fs::read_to_string(path)?.parse()?;

    println!("Compiling Hyperscan databases with {} patterns.", patterns.len());

    Ok((build_database(&patterns)?, build_database(&patterns)?))
}

fn build_database<B: Builder<Err = hyperscan::Error>, T: Mode>(builder: &B) -> Result<Database<T>> {
    let now = Instant::now();

    let db = builder.build::<T>()?;

    println!(
        "compile `{}` mode database in {} ms",
        T::NAME,
        now.elapsed().as_millis()
    );

    Ok(db)
}

// Key for identifying a stream in our pcap input data, using data from its IP
// headers.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Session {
    proto: u8,
    src: SocketAddrV4,
    dst: SocketAddrV4,
}

impl Session {
    fn new(ipv4: &Ipv4Packet) -> Session {
        let mut c = io::Cursor::new(ipv4.payload());
        let src_port = c.read_u16::<BigEndian>().unwrap();
        let dst_port = c.read_u16::<BigEndian>().unwrap();

        Session {
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
    sessions: HashMap<Session, usize>,

    /// Hyperscan compiled database (streaming mode)
    streaming_db: StreamingDatabase,

    /// Hyperscan compiled database (block mode)
    block_db: BlockDatabase,

    /// Hyperscan temporary scratch space (used in both modes)
    scratch: Scratch,

    // Vector of Hyperscan stream state (used in streaming mode)
    streams: Vec<Stream>,

    // Count of matches found during scanning
    match_count: AtomicUsize,
}

impl Benchmark {
    fn new(streaming_db: StreamingDatabase, block_db: BlockDatabase) -> Result<Benchmark> {
        let mut s = streaming_db.alloc_scratch()?;

        block_db.realloc_scratch(&mut s)?;

        Ok(Benchmark {
            packets: Vec::new(),
            stream_ids: Vec::new(),
            sessions: HashMap::new(),
            streaming_db: streaming_db,
            block_db: block_db,
            scratch: s,
            streams: Vec::new(),
            match_count: AtomicUsize::new(0),
        })
    }

    fn decode_packet(packet: &pcap::Packet) -> Option<(Session, Vec<u8>)> {
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

                Some((Session::new(&ipv4), Vec::from(&payload[data_off..])))
            }

            IpNextHeaderProtocols::Udp => {
                let udp = UdpPacket::new(&ipv4.payload()).unwrap();

                Some((Session::new(&ipv4), Vec::from(udp.payload())))
            }
            _ => None,
        }
    }

    fn read_streams<P: AsRef<Path>>(&mut self, path: P) -> Result<(), pcap::Error> {
        let mut capture = pcap::Capture::from_file(path)?;

        while let Ok(ref packet) = capture.next_packet() {
            if let Some((key, payload)) = Self::decode_packet(&packet) {
                if payload.len() > 0 {
                    let stream_id = match self.sessions.get(&key) {
                        Some(&id) => id,
                        None => {
                            let id = self.sessions.len();

                            assert!(self.sessions.insert(key, id).is_none());

                            id
                        }
                    };

                    self.stream_ids.push(stream_id);
                    self.packets.push(Box::new(payload));
                }
            }
        }

        println!(
            "read {} packets in {} sessions",
            self.packets.len(),
            self.stream_ids.len(),
        );

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

    // Open a Hyperscan stream for each stream in stream_ids
    fn open_streams(&mut self) -> Result<()> {
        self.streams = iter::repeat_with(|| self.streaming_db.open_stream())
            .take(self.sessions.len())
            .collect::<hyperscan::Result<Vec<_>>>()?;

        Ok(())
    }

    // Close all open Hyperscan streams (potentially generating any end-anchored matches)
    fn close_streams(&mut self) -> Result<()> {
        for stream in self.streams.drain(..) {
            let match_count = &self.match_count;
            stream
                .close(&self.scratch, |_, _, _, _| {
                    match_count.fetch_add(1, Ordering::Relaxed);

                    Matching::Continue
                })
                .with_context(|| "close stream")?;
        }

        Ok(())
    }

    fn reset_streams(&mut self) -> Result<()> {
        for ref stream in &self.streams {
            stream
                .reset(&self.scratch, |_, _, _, _| {
                    self.match_count.fetch_add(1, Ordering::Relaxed);

                    Matching::Continue
                })
                .with_context(|| "reset stream")?;
        }

        Ok(())
    }

    // Scan each packet (in the ordering given in the PCAP file)
    // through Hyperscan using the streaming interface.
    fn scan_streams(&mut self) -> Result<()> {
        for (i, ref packet) in self.packets.iter().enumerate() {
            let ref stream = self.streams[self.stream_ids[i]];

            stream
                .scan(packet.as_ref().as_slice(), &self.scratch, |_, _, _, _| {
                    self.match_count.fetch_add(1, Ordering::Relaxed);

                    Matching::Continue
                })
                .with_context(|| "scan packet")?;
        }

        Ok(())
    }

    // Scan each packet (in the ordering given in the PCAP file)
    // through Hyperscan using the block-mode interface.
    fn scan_block(&mut self) -> Result<()> {
        for ref packet in &self.packets {
            self.block_db
                .scan(packet.as_ref().as_slice(), &self.scratch, |_, _, _, _| {
                    self.match_count.fetch_add(1, Ordering::Relaxed);

                    Matching::Continue
                })
                .with_context(|| "scan packet")?;
        }

        Ok(())
    }

    // Display some information about the compiled database and scanned data.
    fn display_stats(&self) -> Result<()> {
        let num_packets = self.packets.len();
        let num_streams = self.sessions.len();
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
        println!(
            "Streaming mode Hyperscan database size    : {} bytes.",
            self.streaming_db.size()?
        );
        println!(
            "Block mode Hyperscan database size        : {} bytes.",
            self.block_db.size()?
        );
        println!(
            "Streaming mode Hyperscan stream state size: {} bytes (per stream).",
            self.streaming_db.stream_size()?
        );

        Ok(())
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "simplegrep", about = "An example search a given input file for a pattern.")]
struct Opt {
    /// repeat times
    #[structopt(short = "n", default_value = "1")]
    repeats: usize,

    /// pattern file
    #[structopt(parse(from_os_str))]
    pattern_file: PathBuf,

    /// pcap file
    #[structopt(parse(from_os_str))]
    pcap_file: PathBuf,
}

// Main entry point.
fn main() -> Result<()> {
    let Opt {
        repeats,
        pattern_file,
        pcap_file,
    } = Opt::from_args();

    // Read our pattern set in and build Hyperscan databases from it.
    println!("Pattern file: {:?}", pattern_file);

    let (streaming_db, block_db) = match read_databases(pattern_file) {
        Ok((streaming_db, block_db)) => (streaming_db, block_db),
        Err(err) => {
            eprintln!("ERROR: Unable to parse and compile patterns: {}\n", err);
            exit(-1);
        }
    };

    // Read our input PCAP file in
    let mut bench = Benchmark::new(streaming_db, block_db)?;

    println!("PCAP input file: {:?}", pcap_file);

    if let Err(err) = bench.read_streams(pcap_file) {
        eprintln!("Unable to read packets from PCAP file. Exiting. {}\n", err);
        exit(-1);
    }

    if repeats != 1 {
        println!("Repeating PCAP scan {} times.", repeats);
    }

    bench.display_stats()?;

    // Streaming mode scans.
    let mut streaming_scan = Duration::from_secs(0);
    let mut streaming_open_close = Duration::from_secs(0);

    for i in 0..repeats {
        if i == 0 {
            // Open streams.
            let now = Instant::now();
            bench.open_streams()?;
            streaming_open_close = streaming_open_close + now.elapsed();
        } else {
            // Reset streams.
            let now = Instant::now();
            bench.reset_streams()?;
            streaming_open_close = streaming_open_close + now.elapsed();
        }

        // Scan all our packets in streaming mode.
        let now = Instant::now();
        bench.scan_streams()?;
        streaming_scan = streaming_scan + now.elapsed();
    }

    // Close streams.
    let now = Instant::now();
    bench.close_streams()?;
    streaming_open_close = streaming_open_close + now.elapsed();

    // Collect data from streaming mode scans.
    let bytes = bench.bytes();
    let total_bytes = (bytes * 8 * repeats) as f64;
    let tput_stream_scanning = total_bytes * 1000.0 / streaming_scan.as_millis() as f64;
    let tput_stream_overhead = total_bytes * 1000.0 / (streaming_scan + streaming_open_close).as_millis() as f64;
    let matches_stream = bench.matches();
    let match_rate_stream = (matches_stream as f64) / ((bytes * repeats) as f64 / 1024.0);

    // Scan all our packets in block mode.
    bench.clear_matches();
    let now = Instant::now();
    for _ in 0..repeats {
        bench.scan_block()?;
    }
    let scan_block = now.elapsed();

    // Collect data from block mode scans.
    let tput_block_scanning = total_bytes * 1000.0 / scan_block.as_millis() as f64;
    let matches_block = bench.matches();
    let match_rate_block = (matches_block as f64) / ((bytes * repeats) as f64 / 1024.0);

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

    Ok(())
}
