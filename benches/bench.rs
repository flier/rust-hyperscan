#![feature(test)]
#![feature(concat_idents)]

#[macro_use]
extern crate lazy_static;
extern crate rand;

extern crate hyperscan;
extern crate regex;

#[cfg(test)]
extern crate test;

#[cfg(test)]
mod bench {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use hyperscan::*;
    use rand::{thread_rng, Rng};
    use regex::Regex;

    use test::{black_box, Bencher};

    const RE_SIMPLE: &str = "flier";
    const RE_EMAIL: &str = r"[\w\d._%+-]+@[\w\d.-]+\.[\w]{2,}";
    const RE_RFC5322: &str = r"[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?";

    const SHORT_DATA: &str = "Flier Lu <flier.lu@gmail.com>";

    lazy_static! {
        static ref _4K_DATA: String = {
            thread_rng()
                .gen_ascii_chars()
                .take(4096)
                .chain(SHORT_DATA.chars())
                .collect()
        };
    }

    extern "C" fn on_matched(_id: u32, _from: u64, _to: u64, _flags: u32, matched: &AtomicUsize) -> u32 {
        matched.fetch_add(1, Ordering::Relaxed);
        0
    }

    struct HsBencher {}

    impl HsBencher {
        #[cfg(feature = "bench_parse")]
        fn bench_parse(b: &mut Bencher, r: &str) {
            b.iter(|| {
                let _: Pattern = r.parse().unwrap();
            })
        }

        #[cfg(feature = "bench_compile")]
        fn bench_compile<T: DatabaseType>(b: &mut Bencher, r: &str) {
            let p: Pattern = r.parse().unwrap();
            let platform = PlatformInfo::populate().ok();

            b.iter(|| {
                let _: RawDatabase<T> = p.build_for_platform(platform.as_ref()).unwrap();
            })
        }

        #[cfg(feature = "bench_scan")]
        fn bench_block_scan(b: &mut Bencher, r: &str, data: &str, times: usize) {
            let p: Pattern = r.parse().unwrap();
            let platform = PlatformInfo::populate().ok();
            let db: BlockDatabase = p.build_for_platform(platform.as_ref()).unwrap();
            let mut s = db.alloc().unwrap();
            let mut bytes = 0;

            b.iter(|| {
                let matched = AtomicUsize::new(0);
                let n = black_box(times);

                (0..n).fold(0, |_, _| {
                    let _ = db.scan(data, 0, &mut s, Some(on_matched), Some(&matched)).unwrap();
                    0
                });

                bytes += (data.len() * n) as u64;

                assert!(matched.into_inner() >= n);
            });

            b.bytes = bytes;
        }
    }

    struct RegexBencher {}

    impl RegexBencher {
        #[cfg(feature = "bench_compile")]
        fn bench_compile(b: &mut Bencher, r: &str) {
            b.iter(|| {
                let _ = Regex::new(r).unwrap();
            })
        }

        #[cfg(feature = "bench_scan")]
        fn bench_scan(b: &mut Bencher, r: &str, data: &str, times: usize) {
            let r = Regex::new(r).unwrap();
            let mut bytes = 0;
            let n = black_box(times);

            b.iter(|| {
                assert_eq!((0..n).fold(0, |matched, _| matched + r.find_iter(data).count()), n);

                bytes += (data.len() * times) as u64;
            });

            b.bytes = bytes;
        }
    }

    #[cfg(feature = "bench_parse")]
    #[bench]
    fn parse_simple_pattern(b: &mut Bencher) {
        HsBencher::bench_parse(b, RE_SIMPLE);
    }

    #[cfg(feature = "bench_parse")]
    #[bench]
    fn parse_email_pattern(b: &mut Bencher) {
        HsBencher::bench_parse(b, RE_EMAIL);
    }

    #[cfg(feature = "bench_parse")]
    #[bench]
    fn parse_rfc5322_pattern(b: &mut Bencher) {
        HsBencher::bench_parse(b, RE_RFC5322);
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_simple_pattern_as_block_database(b: &mut Bencher) {
        HsBencher::bench_compile::<Block>(b, RE_SIMPLE);
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_simple_pattern_as_vectored_database(b: &mut Bencher) {
        HsBencher::bench_compile::<Vectored>(b, RE_SIMPLE);
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_simple_pattern_as_streaming_database(b: &mut Bencher) {
        HsBencher::bench_compile::<Streaming>(b, RE_SIMPLE);
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_email_pattern_as_block_database(b: &mut Bencher) {
        HsBencher::bench_compile::<Block>(b, RE_EMAIL);
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_email_pattern_as_vectored_database(b: &mut Bencher) {
        HsBencher::bench_compile::<Vectored>(b, RE_EMAIL);
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_email_pattern_as_streaming_database(b: &mut Bencher) {
        HsBencher::bench_compile::<Streaming>(b, RE_EMAIL);
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_rfc5322_pattern_as_block_database(b: &mut Bencher) {
        HsBencher::bench_compile::<Block>(b, RE_RFC5322);
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_rfc5322_pattern_as_vectored_database(b: &mut Bencher) {
        HsBencher::bench_compile::<Vectored>(b, RE_RFC5322);
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_rfc5322_pattern_as_streaming_database(b: &mut Bencher) {
        HsBencher::bench_compile::<Streaming>(b, RE_RFC5322);
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_simple_pattern_with_regex(b: &mut Bencher) {
        RegexBencher::bench_compile(b, RE_SIMPLE);
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_email_pattern_with_regex(b: &mut Bencher) {
        RegexBencher::bench_compile(b, RE_EMAIL);
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_rfc5322_pattern_with_regex(b: &mut Bencher) {
        RegexBencher::bench_compile(b, RE_RFC5322);
    }

    #[cfg(feature = "bench_scan")]
    #[bench]
    fn scan_simple_pattern_as_block_database_1000_times(b: &mut Bencher) {
        HsBencher::bench_block_scan(b, RE_SIMPLE, SHORT_DATA, 1000);
    }

    #[cfg(feature = "bench_scan")]
    #[bench]
    fn scan_email_pattern_as_block_database_1000_times(b: &mut Bencher) {
        HsBencher::bench_block_scan(b, RE_EMAIL, SHORT_DATA, 1000);
    }

    #[cfg(feature = "bench_scan")]
    #[bench]
    fn scan_rfc5322_pattern_as_block_database_1000_times(b: &mut Bencher) {
        HsBencher::bench_block_scan(b, RE_RFC5322, SHORT_DATA, 1000);
    }

    #[cfg(feature = "bench_scan")]
    #[bench]
    fn scan_simple_pattern_as_block_database_with_4k_data_1000_times(b: &mut Bencher) {
        HsBencher::bench_block_scan(b, RE_SIMPLE, _4K_DATA.as_str(), 1000);
    }

    #[cfg(feature = "bench_scan")]
    #[bench]
    fn scan_email_pattern_as_block_database_with_4k_data_1000_times(b: &mut Bencher) {
        HsBencher::bench_block_scan(b, RE_EMAIL, _4K_DATA.as_str(), 1000);
    }

    #[cfg(feature = "bench_scan")]
    #[bench]
    fn scan_rfc5322_pattern_as_block_database_with_4k_data_1000_times(b: &mut Bencher) {
        HsBencher::bench_block_scan(b, RE_RFC5322, _4K_DATA.as_str(), 1000);
    }

    #[cfg(feature = "bench_scan")]
    #[bench]
    fn scan_simple_pattern_with_regex_1000_times(b: &mut Bencher) {
        RegexBencher::bench_scan(b, RE_SIMPLE, SHORT_DATA, 1000);
    }

    #[cfg(feature = "bench_scan")]
    #[bench]
    fn scan_email_pattern_with_regex_1000_times(b: &mut Bencher) {
        RegexBencher::bench_scan(b, RE_EMAIL, SHORT_DATA, 1000);
    }

    #[cfg(feature = "bench_scan")]
    #[bench]
    fn scan_rfc5322_pattern_with_regex_1000_times(b: &mut Bencher) {
        RegexBencher::bench_scan(b, RE_RFC5322, SHORT_DATA, 1000);
    }

    #[cfg(feature = "bench_scan")]
    #[bench]
    fn scan_simple_pattern_with_regex_and_4k_data_1000_times(b: &mut Bencher) {
        RegexBencher::bench_scan(b, RE_SIMPLE, _4K_DATA.as_str(), 1000);
    }

    #[cfg(feature = "bench_scan")]
    #[bench]
    fn scan_email_pattern_with_regex_and_4k_data_1000_times(b: &mut Bencher) {
        RegexBencher::bench_scan(b, RE_EMAIL, _4K_DATA.as_str(), 1000);
    }

    #[cfg(feature = "bench_scan")]
    #[bench]
    fn scan_rfc5322_pattern_with_regex_and_4k_data_1000_times(b: &mut Bencher) {
        RegexBencher::bench_scan(b, RE_RFC5322, _4K_DATA.as_str(), 1000);
    }
}
