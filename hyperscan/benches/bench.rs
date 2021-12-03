use std::collections::HashMap;
use std::iter;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use lazy_static::lazy_static;

const KB: usize = 1024;
const MB: usize = 1024 * KB;

lazy_static! {
    static ref BENCH_DATA: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("Easy0", "ABCDEFGHIJKLMNOPQRSTUVWXYZ$");
        m.insert("Easy0i", "(?i)ABCDEFGHIJklmnopqrstuvwxyz$");
        m.insert("Easy1", "A[AB]B[BC]C[CD]D[DE]E[EF]F[FG]G[GH]H[HI]I[IJ]J$");
        m.insert("Medium", "[XYZ]ABCDEFGHIJKLMNOPQRSTUVWXYZ$");
        m.insert("Hard", "[ -~]*ABCDEFGHIJKLMNOPQRSTUVWXYZ$");
        m.insert("Hard1", "ABCD|CDEF|EFGH|GHIJ|IJKL|KLMN|MNOP|OPQR|QRST|STUV|UVWX|WXYZ");
        m
    };
    static ref BENCH_SIZE: Vec<usize> = vec![16, 32, KB, 32 * KB, MB, 32 * MB];
    static ref BENCH_TEXT: Vec<u8> = {
        let mut x: u32 = !0;

        iter::from_fn(|| {
            x += x;
            x ^= 1;

            if (x as isize) < 0 {
                x ^= 0x8888_8eef
            }

            if x % 31 == 0 {
                Some(b'\n')
            } else {
                Some((x % (0x7E + 1 - 0x20) + 0x20) as u8)
            }
        })
        .take(32 * MB)
        .collect()
    };
}

fn hyperscan_bench(c: &mut Criterion) {
    use hyperscan::{prelude::*, BlockMode, PatternFlags};

    let mut group = c.benchmark_group("hyperscan");

    for (&name, &expr) in BENCH_DATA.iter() {
        let pat = Pattern::with_flags(expr, PatternFlags::SOM_LEFTMOST | PatternFlags::MULTILINE).unwrap();
        let db = pat.build::<BlockMode>().unwrap();
        let s = db.alloc_scratch().unwrap();

        for &size in BENCH_SIZE.iter() {
            let text = BENCH_TEXT.get(..size).unwrap();

            group.throughput(Throughput::Bytes(text.len() as u64));
            group.bench_with_input(BenchmarkId::new(name, size), &(text), |b, text| {
                b.iter(|| db.scan(text, &s, Matching::Terminate).unwrap())
            });
        }
    }

    group.finish();
}

#[cfg(feature = "chimera")]
fn chimera_bench(c: &mut Criterion) {
    use hyperscan::chimera::{prelude::*, Flags};

    let mut group = c.benchmark_group("chimera");

    for (&name, &expr) in BENCH_DATA.iter() {
        let pat = Pattern::with_flags(expr, Flags::MULTILINE);
        let db = pat.build().unwrap();
        let s = db.alloc_scratch().unwrap();

        for &size in BENCH_SIZE.iter() {
            let text = BENCH_TEXT.get(..size).unwrap();

            group.throughput(Throughput::Bytes(text.len() as u64));
            group.bench_with_input(BenchmarkId::new(name, size), &(text), |b, text| {
                b.iter(|| db.scan(text, &s, Matching::Terminate, Matching::Terminate).unwrap())
            });
        }
    }

    group.finish();
}

#[cfg(not(feature = "chimera"))]
fn chimera_bench(c: &mut Criterion) {}

fn regex_bench(c: &mut Criterion) {
    use std::str;

    use regex::RegexBuilder;

    let mut group = c.benchmark_group("regex");

    for (&name, &expr) in BENCH_DATA.iter() {
        let re = RegexBuilder::new(expr).multi_line(true).build().unwrap();

        for &size in BENCH_SIZE.iter() {
            let text = unsafe { str::from_utf8_unchecked(BENCH_TEXT.get(..size).unwrap()) };

            group.throughput(Throughput::Bytes(text.len() as u64));
            group.bench_with_input(BenchmarkId::new(name, size), &(text), |b, text| {
                b.iter(|| re.find_iter(text).collect::<Vec<_>>())
            });
        }
    }

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = hyperscan_bench, chimera_bench, regex_bench
}

criterion_main!(benches);
