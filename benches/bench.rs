#![feature(test)]
#![feature(concat_idents)]

extern crate hyperscan;
extern crate regex;

#[cfg(test)]
extern crate test;

#[cfg(test)]
mod bench {
    use std::sync::atomic::{Ordering, AtomicUsize};

    use hyperscan::{self as hs, ScratchAllocator, BlockScanner, DatabaseBuilder};

    use test::{Bencher, black_box};

    const RE_SIMPLE: &str = "flier";
    const RE_EMAIL: &str = r"[\w\d._%+-]+@[\w\d.-]+\.[\w]{2,}";
    const RE_RFC5322: &str = r"[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?";

    const SHORT_DATA: &str = "Flier Lu <flier.lu@gmail.com>";

    extern "C" fn on_matched(_: u32, _: u64, _: u64, _: u32, matched: &AtomicUsize) -> u32 {
        matched.fetch_add(1, Ordering::Relaxed);
        0
    }

    #[cfg(feature = "bench_parse")]
    #[bench]
    fn parse_simple_pattern(b: &mut Bencher) {
        b.iter(|| { let _: hs::Pattern = RE_SIMPLE.parse().unwrap(); })
    }

    #[cfg(feature = "bench_parse")]
    #[bench]
    fn parse_email_pattern(b: &mut Bencher) {
        b.iter(|| { let _: hs::Pattern = RE_EMAIL.parse().unwrap(); })
    }

    #[cfg(feature = "bench_parse")]
    #[bench]
    fn parse_rfc5322_pattern(b: &mut Bencher) {
        b.iter(|| { let _: hs::Pattern = RE_RFC5322.parse().unwrap(); })
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_simple_pattern_as_block_database(b: &mut Bencher) {
        let p: hs::Pattern = RE_SIMPLE.parse().unwrap();
        let platform = hs::PlatformInfo::populate().ok();

        b.iter(|| { let _: hs::BlockDatabase = p.build_for_platform(platform.as_ref()).unwrap(); })
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_simple_pattern_as_vectored_database(b: &mut Bencher) {
        let p: hs::Pattern = RE_SIMPLE.parse().unwrap();
        let platform = hs::PlatformInfo::populate().ok();

        b.iter(|| { let _: hs::VectoredDatabase = p.build_for_platform(platform.as_ref()).unwrap(); })
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_simple_pattern_as_streaming_database(b: &mut Bencher) {
        let p: hs::Pattern = RE_SIMPLE.parse().unwrap();
        let platform = hs::PlatformInfo::populate().ok();

        b.iter(|| { let _: hs::StreamingDatabase = p.build_for_platform(platform.as_ref()).unwrap(); })
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_email_pattern_as_block_database(b: &mut Bencher) {
        let p: hs::Pattern = RE_EMAIL.parse().unwrap();
        let platform = hs::PlatformInfo::populate().ok();

        b.iter(|| { let _: hs::BlockDatabase = p.build_for_platform(platform.as_ref()).unwrap(); })
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_email_pattern_as_vectored_database(b: &mut Bencher) {
        let p: hs::Pattern = RE_EMAIL.parse().unwrap();
        let platform = hs::PlatformInfo::populate().ok();

        b.iter(|| { let _: hs::VectoredDatabase = p.build_for_platform(platform.as_ref()).unwrap(); })
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_email_pattern_as_streaming_database(b: &mut Bencher) {
        let p: hs::Pattern = RE_EMAIL.parse().unwrap();
        let platform = hs::PlatformInfo::populate().ok();

        b.iter(|| { let _: hs::StreamingDatabase = p.build_for_platform(platform.as_ref()).unwrap(); })
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_rfc5322_pattern_as_block_database(b: &mut Bencher) {
        let p: hs::Pattern = RE_RFC5322.parse().unwrap();
        let platform = hs::PlatformInfo::populate().ok();

        b.iter(|| { let _: hs::BlockDatabase = p.build_for_platform(platform.as_ref()).unwrap(); })
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_rfc5322_pattern_as_vectored_database(b: &mut Bencher) {
        let p: hs::Pattern = RE_RFC5322.parse().unwrap();
        let platform = hs::PlatformInfo::populate().ok();

        b.iter(|| { let _: hs::VectoredDatabase = p.build_for_platform(platform.as_ref()).unwrap(); })
    }

    #[cfg(feature = "bench_compile")]
    #[bench]
    fn compile_rfc5322_pattern_as_streaming_database(b: &mut Bencher) {
        let p: hs::Pattern = RE_RFC5322.parse().unwrap();
        let platform = hs::PlatformInfo::populate().ok();

        b.iter(|| { let _: hs::StreamingDatabase = p.build_for_platform(platform.as_ref()).unwrap(); })
    }

    #[cfg(feature = "bench_scan")]
    #[bench]
    fn scan_simple_pattern_as_block_database_100_times(b: &mut Bencher) {
        let p: hs::Pattern = RE_SIMPLE.parse().unwrap();
        let platform = hs::PlatformInfo::populate().ok();
        let db: hs::BlockDatabase = p.build_for_platform(platform.as_ref()).unwrap();
        let mut s = db.alloc().unwrap();
        let matched = AtomicUsize::new(0);

        let n = black_box(100);

        b.iter(|| {
            (0..n).fold(0, |_, _| {
                db.scan(SHORT_DATA, 0, &mut s, Some(on_matched), Some(&matched)).unwrap();
                0
            })
        });

        assert!(matched.into_inner() > 0);
    }

    #[cfg(feature = "bench_scan")]
    #[bench]
    fn scan_email_pattern_as_block_database_100_times(b: &mut Bencher) {
        let p: hs::Pattern = RE_EMAIL.parse().unwrap();
        let platform = hs::PlatformInfo::populate().ok();
        let db: hs::BlockDatabase = p.build_for_platform(platform.as_ref()).unwrap();
        let mut s = db.alloc().unwrap();
        let matched = AtomicUsize::new(0);

        let n = black_box(100);

        b.iter(|| {
            (0..n).fold(0, |_, _| {
                db.scan(SHORT_DATA, 0, &mut s, Some(on_matched), Some(&matched)).unwrap();
                0
            })
        });

        assert!(matched.into_inner() > 0);
    }

    #[cfg(feature = "bench_scan")]
    #[bench]
    fn scan_rfc5322_pattern_as_block_database_100_times(b: &mut Bencher) {
        let p: hs::Pattern = RE_RFC5322.parse().unwrap();
        let platform = hs::PlatformInfo::populate().ok();
        let db: hs::BlockDatabase = p.build_for_platform(platform.as_ref()).unwrap();
        let mut s = db.alloc().unwrap();
        let matched = AtomicUsize::new(0);

        let n = black_box(100);

        b.iter(|| {
            (0..n).fold(0, |_, _| {
                db.scan(SHORT_DATA, 0, &mut s, Some(on_matched), Some(&matched)).unwrap();
                0
            })
        });

        assert!(matched.into_inner() > 0);
    }
}
