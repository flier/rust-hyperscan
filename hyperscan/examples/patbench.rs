//! Hyperscan pattern benchmarker.
//!
//! This program allows users to detect which signatures may be the most
//! expensive in a set of patterns. It is designed for use with small to medium
//! pattern set sizes (e.g. 5-500). If used with very large pattern sets it may
//! take a very long time - the number of recompiles done is g * O(lg2(n)) where
//! g is the number of generations and n is the number of patterns (assuming
//! that n >> g).
//!
//! This utility will return a cumulative series of removed patterns. The first
//! generation will find and remove a single pattern. The second generation will
//! begin with the first pattern removed and find another pattern to remove,
//! etc. So if we have 100 patterns and 15 generations, the final generation's
//! score will be a run over 85 patterns.
//!
//! This utility is probabilistic. It is possible that the pattern removed in a
//! generation is not a particularly expensive pattern. To reduce noise in the
//! results use 'taskset' and set the number of repeats to a level that still
//! completes in reasonable time (this will reduce the effect of random
//! measurement noise).
//!
//! The criterion for performance can be altered by use of the -C<x> flag where
//! <x> can be t,r,s,c,b, selecting pattern matching throughput, scratch size,
//! stream state size (only available in streaming mode), compile time and
//! bytecode size respectively.
//!
//! This utility will also not produce good results if all the patterns are
//! roughly equally expensive.
//!
//! Factor Group Size:
//!
//! If there are multiple expensive patterns that are very similar on the
//! left-hand-side or identical, this utility will typically not find these
//! groups unless the -F flag is used to search for a group size that is equal
//! to or larger than the size of the group of similar patterns.
//!
//! Otherwise, removing a portion of the similar patterns will have no or almost
//! no effect, and the search procedure used relies on the ability to remove all
//! of the similar patterns in at least one search case, something which will
//! only happen if the factor_group_size is large enough.
//!
//! This alters the operation of our tool so that instead of trying to find the
//! single pattern whose removal has the most effect by binary search (the
//! default with factor_group_size == 1), we attempt to find the N patterns
//! whose removal has the most effect by searching over N+1 evenly sized groups,
//! removing only 1/(N+1) of the search signatures per iteration.
//!
//! Note that the number of recompiles done greatly increases with increased
//! factor group size.  For example, with factor_group_size = 1, we do g * 2 *
//! lg2(n) recompiles, while with factor_group_size = 4, we do g * 4 *
//! log(5/4)(n). Informally the number of generations we require goes up as we
//! eliminate a smaller number of signatures and the we have to do more work per
//! generation.
//!
//!
//! Build instructions:
//!
//!     cargo build --example patbench
//!
//! Usage:
//!
//!     ./patbench [ -n repeats] [ -G generations] [ -C criterion ]
//!             [ -F factor_group_size ] [ -N | -S ] <pattern file> <pcap file>
//!
//!     -n repeats sets the number of times the PCAP is repeatedly scanned
//!        with the pattern
//!     -G generations sets the number of generations that the algorithm is
//!        run for
//!     -N sets non-streaming mode, -S sets streaming mode (default)
//!     -F sets the factor group size (must be >0); this allows the detection
//!        of multiple interacting factors
//!
//!     -C sets the "criterion", which can be either:
//!          t  throughput (the default) - this requires a pcap file
//!          r  scratch size
//!          s  stream state size
//!          c  compile time
//!          b  bytecode size
//!
//! We recommend the use of a utility like 'taskset' on multiprocessor hosts to
//! lock execution to a single processor: this will remove processor migration
//! by the scheduler as a source of noise in the results.
//!
//!
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::net::SocketAddrV4;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Instant;

use anyhow::{anyhow, bail, Context, Error, Result};
use byteorder::{BigEndian, ReadBytesExt};
use derive_more::{Deref, Display, Index};
use either::Either;
use pnet::packet::{
    ethernet::{EtherTypes, EthernetPacket},
    ip::IpNextHeaderProtocols,
    ipv4::Ipv4Packet,
    udp::UdpPacket,
    Packet, PrimitiveValues,
};
use rand::seq::SliceRandom;
use structopt::StructOpt;

use hyperscan::{prelude::*, Block, Streaming};

#[derive(Clone, Copy, Debug, Display, PartialEq, Eq)]
enum Criterion {
    #[display(fmt = "throughput")]
    Throughput,
    #[display(fmt = "bytecode_size")]
    ByteCodeSize,
    #[display(fmt = "compile_time")]
    CompileTime,
    #[display(fmt = "stream_state")]
    StreamStateSize,
    #[display(fmt = "scratch_size")]
    ScratchSize,
}

impl Criterion {
    fn higher_is_better(self) -> bool {
        self == Criterion::Throughput
    }
}

impl Default for Criterion {
    fn default() -> Self {
        Criterion::Throughput
    }
}

impl FromStr for Criterion {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Criterion::*;

        Ok(match s {
            "t" | "throughput" => Throughput,
            "b" | "bytecode_size" => ByteCodeSize,
            "c" | "compile_time" => CompileTime,
            "s" | "stream_state" => StreamStateSize,
            "r" | "scratch_size" => ScratchSize,
            _ => bail!("Unknown criterion: {}", s),
        })
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "simplegrep", about = "An example search a given input file for a pattern.")]
struct Opt {
    /// sets the number of times the PCAP is repeatedly scanned with the pattern.
    #[structopt(short, default_value = "1")]
    repeats: usize,

    /// generations sets the number of generations that the algorithm is run for.
    #[structopt(short = "G", default_value = "10")]
    gen_max: usize,

    /// sets non-streaming mode
    #[structopt(short = "N")]
    non_streaming: bool,

    /// sets the factor group size (must be >0); this allows the detection of multiple interacting factors
    #[structopt(short = "F", default_value = "1")]
    factor_max: usize,

    /// sets the criterion, which can be either:
    ///     `t`: throughput (the default) - this requires a pcap file,
    ///     `r`: scratch size,
    ///     `s`: stream state size,
    ///     `c`: compile time,
    ///     `b`: bytecode size
    #[structopt(short = "C", default_value)]
    criterion: Criterion,

    /// pattern file
    #[structopt(parse(from_os_str))]
    pattern_file: PathBuf,

    /// pcap file
    #[structopt(parse(from_os_str))]
    pcap_file: Option<PathBuf>,
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

    fn decode(packet: &pcap::Packet) -> Option<(Session, Vec<u8>)> {
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
}

/// Class wrapping all state associated with the benchmark
#[derive(Default)]
struct Benchmark {
    /// Packet data to be scanned
    packets: Vec<Vec<u8>>,

    /// Stream ID for each packet
    stream_ids: Vec<usize>,

    /// Map used to construct stream_ids
    sessions: HashMap<Session, usize>,

    // Vector of Hyperscan stream state (used in streaming mode)
    streams: Vec<Stream>,

    // Count of matches found during scanning
    matches: usize,
}

const IP_FLAG_MF: u8 = 1;

impl Benchmark {
    fn new() -> Self {
        Default::default()
    }

    fn read_streams<P: AsRef<Path>>(&mut self, path: P) -> Result<(), pcap::Error> {
        let mut capture = pcap::Capture::from_file(path)?;

        while let Ok(ref packet) = capture.next_packet() {
            if let Some((key, payload)) = Session::decode(&packet) {
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
                    self.packets.push(payload);
                }
            }
        }

        Ok(())
    }

    // Clear the number of matches found.
    fn clear_matches(&mut self) {
        self.matches = 0;
    }

    // Return the number of bytes scanned
    fn bytes(&self) -> usize {
        self.packets.iter().fold(0, |bytes, p| bytes + p.len())
    }

    /// Open a Hyperscan stream for each stream in stream_ids
    fn open_streams(&mut self, db: &StreamingDatabase) -> Result<()> {
        self.streams = (0..self.sessions.len())
            .map(|_| db.open_stream())
            .collect::<hyperscan::Result<Vec<_>>>()?;

        Ok(())
    }

    /// Close all open Hyperscan streams (potentially generating any end-anchored matches)
    fn close_streams(&mut self, scratch: &Scratch) -> Result<()> {
        let matches = &mut self.matches;

        for stream in self.streams.drain(..) {
            stream.close(&scratch, |_, _, _, _| {
                *matches += 1;
                Matching::Continue
            })?;
        }

        Ok(())
    }

    /// Scan each packet (in the ordering given in the PCAP file) through Hyperscan using the streaming interface.
    fn scan_streams(&mut self, scratch: &Scratch) -> Result<()> {
        let matches = &mut self.matches;

        for (i, ref packet) in self.packets.iter().enumerate() {
            let ref stream = self.streams[self.stream_ids[i]];

            stream.scan(&packet, &scratch, |_, _, _, _| {
                *matches += 1;
                Matching::Continue
            })?;
        }

        Ok(())
    }

    /// Scan each packet (in the ordering given in the PCAP file) through
    /// Hyperscan using the block-mode interface.
    fn scan_block(&mut self, db: &BlockDatabase, scratch: &Scratch) -> Result<()> {
        let matches = &mut self.matches;

        for packet in &self.packets {
            db.scan(packet, &scratch, |_, _, _, _| {
                *matches += 1;
                Matching::Continue
            })?;
        }

        Ok(())
    }
}

#[derive(Deref, Index)]
struct SigData {
    patterns: Patterns,
}

impl SigData {
    fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let patterns = fs::read_to_string(path)
            .with_context(|| "read pattern file")?
            .parse()
            .with_context(|| "parse pattern file")?;

        Ok(Self { patterns })
    }

    fn len(&self) -> usize {
        self.patterns.len()
    }

    fn clone_exclude(&self, excludes: &HashSet<usize>) -> Self {
        Self {
            patterns: self
                .patterns
                .iter()
                .enumerate()
                .filter(|(i, _)| !excludes.contains(i))
                .map(|(_, pattern)| pattern.clone())
                .collect(),
        }
    }
}

fn eval_set(
    bench: &mut Benchmark,
    patterns: &Patterns,
    streaming: bool,
    repeats: usize,
    criterion: Criterion,
    diagnose: bool,
) -> Result<f64> {
    use Criterion::*;

    let now = Instant::now();
    let db = if streaming {
        patterns.build::<Streaming>().map(Either::Left)?
    } else {
        patterns.build::<Block>().map(Either::Right)?
    };
    let compile_time = now.elapsed();
    let scratch = db.as_ref().either(|db| db.alloc_scratch(), |db| db.alloc_scratch())?;

    match criterion {
        ByteCodeSize => db
            .as_ref()
            .either(|db| db.size(), |db| db.size())
            .map(|size| size as f64)
            .with_context(|| "retrieve bytecode size"),
        CompileTime => Ok(compile_time.as_secs_f64()),
        StreamStateSize => db
            .as_ref()
            .either(
                |db| db.stream_size().map(|size| size as f64).with_context(|| "stream size"),
                |_| bail!("Cannot evaluate stream state for block mode compile."),
            )
            .with_context(|| "retrieve stream state size"),
        ScratchSize => scratch
            .size()
            .map(|size| size as f64)
            .with_context(|| "retrieve scratch size"),
        Throughput => {
            bench.clear_matches();

            let now = Instant::now();
            for _ in 0..repeats {
                match db {
                    Either::Left(ref db) => {
                        bench.open_streams(db).with_context(|| "open stream")?;
                        bench.scan_streams(&scratch).with_context(|| "scan stream")?;
                        bench.close_streams(&scratch).with_context(|| "close stream")?;
                    }
                    Either::Right(ref db) => {
                        bench.scan_block(db, &scratch).with_context(|| "scan block")?;
                    }
                }
            }
            let scan_time = now.elapsed();
            let bytes = bench.bytes();
            let throughput = ((bytes * 8 * repeats) as f64) / (scan_time.as_secs_f64() * 1000_000.0);

            if diagnose {
                println!(
                    "Scan time {:.3} sec, Scanned {} bytes, Throughput {:.3} Mbps, Matches {}",
                    scan_time.as_secs_f64(),
                    bytes,
                    throughput,
                    bench.matches
                );
            }

            Ok(throughput)
        }
    }
}

fn main() -> Result<()> {
    let Opt {
        pattern_file,
        pcap_file,
        non_streaming,
        repeats,
        criterion,
        gen_max,
        factor_max,
        ..
    } = Opt::from_args();

    let mut bench = Benchmark::new();

    if criterion == Criterion::Throughput {
        // Read our input PCAP file in
        bench
            .read_streams(pcap_file.as_ref().ok_or(anyhow!("pcap file"))?)
            .with_context(|| "read packets from PCAP file")?;
    }

    println!("Base signatures: {:?}", pattern_file);
    if let Some(ref pcap_file) = pcap_file {
        println!("PCAP input file: {:?}", pcap_file);
        println!("Repeat count: {}", repeats);
    }
    println!("Mode: {}", if non_streaming { "block" } else { "streaming" });

    let sigs = SigData::new(pattern_file)?;
    let mut work_sigs = (0..sigs.len()).collect::<HashSet<_>>();
    let mut excludes = HashSet::new();

    let score_base = eval_set(&mut bench, &sigs, !non_streaming, repeats, criterion, true)?;
    let maximize = criterion.higher_is_better();

    let print_criterion = |score: f64| match criterion {
        Criterion::Throughput => format!("{:.3} Mbps", score),
        Criterion::CompileTime => format!("{:.3} sec", score),
        _ => format!("{} bytes", score as usize),
    };

    println!("Number of signatures: {}", sigs.len());
    println!("Base performance: {}", print_criterion(score_base));

    let generations = gen_max.min((sigs.len() - 1) / factor_max);

    println!("Cutting signatures cumulatively for {} generations", generations);

    let mut rng = rand::thread_rng();

    for gen in 0..generations {
        print!("Generation {} ", gen);

        let mut s = work_sigs.clone();
        let mut best = if maximize { 0.0 } else { 1000000000000.0 };
        let mut count = 0;

        while s.len() > factor_max {
            count += 1;
            print!(".");
            let mut sv = s.iter().cloned().collect::<Vec<_>>();
            sv.shuffle(&mut rng);
            let groups = factor_max + 1;
            for current_group in 0..groups {
                let sz = sv.len();
                let lo = (current_group * sz) / groups;
                let hi = ((current_group + 1) * sz) / groups;

                let s_part1 = &sv[..lo];
                let s_part2 = &sv[hi..];
                let mut s_tmp = s_part1.iter().cloned().collect::<HashSet<_>>();
                s_tmp.extend(s_part2.iter().cloned());
                let mut tmp = s_tmp.clone();
                tmp.extend(excludes.iter().cloned());
                let sigs_tmp = sigs.clone_exclude(&excludes);
                let score = eval_set(&mut bench, &sigs_tmp, !non_streaming, repeats, criterion, false)?;

                if current_group == 0 || (if !maximize { score < best } else { score > best }) {
                    s = s_tmp;
                    best = score;
                }
            }
        }

        for _ in count..16 {
            print!(" ");
        }

        println!(
            "\tPerformance: {} ({:.3}x) after cutting",
            print_criterion(best),
            best / score_base
        );

        // s now has factor_max signatures
        for found in s {
            excludes.insert(found);
            work_sigs.remove(&found);
            print!("{}", sigs[found]);
        }

        println!("");
    }

    Ok(())
}
