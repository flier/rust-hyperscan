# rust-hyperscan
[![Continuous integration](https://github.com/flier/rust-hyperscan/actions/workflows/ci.yml/badge.svg)](https://github.com/flier/rust-hyperscan/actions/workflows/ci.yml) [![Github Pages](https://github.com/flier/rust-hyperscan/actions/workflows/pages.yml/badge.svg?branch=master)](https://github.com/flier/rust-hyperscan/actions/workflows/pages.yml) [![crate](https://img.shields.io/crates/v/hyperscan.svg)](https://crates.io/crates/hyperscan) [![docs](https://img.shields.io/badge/docs-next-brightgreen)](https://flier.github.io/rust-hyperscan/hyperscan/index.html) [![Crates.io](https://img.shields.io/crates/l/hyperscan)](https://spdx.org/licenses/Apache-2.0.html)

[Hyperscan](https://github.com/intel/hyperscan) is a high-performance regular expression matching library.

## Usage

To use, add the following line to Cargo.toml under [dependencies]:

```toml
hyperscan = "0.3"
```

## Examples

```rust
use hyperscan::prelude::*;

fn main() {
    let pattern = pattern! {"test"; CASELESS | SOM_LEFTMOST};
    let db: BlockDatabase = pattern.build().unwrap();
    let scratch = db.alloc_scratch().unwrap();
    let mut matches = vec![];

    db.scan("some test data", &scratch, |id, from, to, flags| {
        println!("found pattern #{} @ [{}, {})", id, from, to);

        matches.push(from..to);

        Matching::Continue
    }).unwrap();

    assert_eq!(matches, vec![5..9]);
}
```

## Features

### Hyperscan v5 API

Starting with Hyperscan v5.0, several new APIs and flags have been introduced.

`rust-hyperscan` uses the latest version of the API by default, providing new features such as `Literal`.

If you want to work with Hyperscan v4.x, you can disable `v5` feature at compile time.

```toml
[dependencies.hyperscan]
version = "0.3"
default-features = false
features = ["full"]
```

### Chimera API

In order to improve regular expression compatibility, Hyperscan v5.0 starts to provide a PCRE-compatible [Chimera](http://intel.github.io/hyperscan/dev-reference/chimera.html) library.

To enable `Chimera` support, you need to manually download PCRE 8.41 or above, unzip to the source directory of Hyperscan 5.x, compile and install it.

```bash
$ cd hyperscan-5.4.0
$ wget https://ftp.pcre.org/pub/pcre/pcre-8.45.tar.gz
$ mkdir pcre
$ tar xvf pcre-8.45.tar.gz --strip-components=1 --directory pcre

$ mkdir build && cd build
$ cmake .. -DCMAKE_INSTALL_PREFIX=`pwd` -DBUILD_STATIC_LIBS=on -G Ninja
$ ninja
$ ninja install
```

Then point to the hyperscan installation directory with the `PKG_CONFIG_PATH` environment variable and enable `chimera` feature.

```bash
$ PKG_CONFIG_PATH=<CMAKE_INSTALL_PREFIX>/lib/pkgconfig cargo build
```

The `chimera` feature should be enabled.

```toml
[dependencies]
hyperscan = { version = "0.3", features = ["chimera"] }
```

Note: The `Chimera` library does not support dynamic library linking mode, `static` feature is automatically enabled when `chimera` is enabled.

### Static Linking Mode

As of version 0.2, `rust-hyperscan` uses dynamic library linking mode by default. If you need link a static library, you can use the `static` feature.

```toml
[dependencies]
hyperscan = { version = "0.3", features = ["static"] }
```

### Hyperscan Runtime

Hyperscan provides [a standalone runtime library](http://intel.github.io/hyperscan/dev-reference/serialization.html#the-runtime-library), which can be used separately. If you don't need to compile regular expressions at runtime, you can reduce the size of the executable using `runtime` mode and get rid of C++ dependencies.

```toml
[dependencies.hyperscan]
version = "0.3"
default-features = false
features = ["runtime"]
```

## Benchmark

To provide a performance comparison, the `Hyperscan`, `Chimera` and `regex` performance testing tools are provided here.

They will use the same test set on different length data.

| Level | Pattern |
|-------|---------|
| Easy0 | ABCDEFGHIJKLMNOPQRSTUVWXYZ$ |
| Easy0i | (?i)ABCDEFGHIJklmnopqrstuvwxyz$ |
| Easy1 | A[AB]B[BC]C[CD]D[DE]E[EF]F[FG]G[GH]H[HI]I[IJ]J$ |
| Medium | [XYZ]ABCDEFGHIJKLMNOPQRSTUVWXYZ$ |
| Hard | [ -~]*ABCDEFGHIJKLMNOPQRSTUVWXYZ$ |
| Hard1 | ABCD\|CDEF\|EFGH\|GHIJ\|IJKL\|KLMN\|MNOP\|OPQR\|QRST\|STUV\|UVWX\|WXYZ |


You can use the [cargo criterion](https://github.com/bheisler/cargo-criterion) command to run benchmark on your environment.


```bash
$ cargo criterion --features chimera
    Finished bench [optimized] target(s) in 0.24s
hyperscan/Hard/16       time:   [7.8000 ns 7.8566 ns 7.9212 ns]
                        thrpt:  [1.8812 GiB/s 1.8967 GiB/s 1.9104 GiB/s]
hyperscan/Hard/32       time:   [53.495 ns 53.668 ns 53.850 ns]
                        thrpt:  [566.71 MiB/s 568.64 MiB/s 570.48 MiB/s]
hyperscan/Hard/1024     time:   [91.912 ns 95.150 ns 101.64 ns]
                        thrpt:  [9.3826 GiB/s 10.023 GiB/s 10.376 GiB/s]
hyperscan/Hard/32768    time:   [1.1840 us 1.1917 us 1.2015 us]
                        thrpt:  [25.400 GiB/s 25.608 GiB/s 25.776 GiB/s]
hyperscan/Hard/1048576  time:   [35.849 us 35.981 us 36.133 us]
                        thrpt:  [27.027 GiB/s 27.141 GiB/s 27.241 GiB/s]
hyperscan/Hard/33554432 time:   [2.1729 ms 2.1893 ms 2.2075 ms]
                        thrpt:  [14.156 GiB/s 14.274 GiB/s 14.381 GiB/s]
hyperscan/Medium/16     time:   [7.8242 ns 7.9619 ns 8.1801 ns]
                        thrpt:  [1.8216 GiB/s 1.8716 GiB/s 1.9045 GiB/s]
hyperscan/Medium/32     time:   [71.114 ns 71.583 ns 72.090 ns]
                        thrpt:  [423.33 MiB/s 426.32 MiB/s 429.13 MiB/s]
hyperscan/Medium/1024   time:   [110.53 ns 111.09 ns 111.75 ns]
                        thrpt:  [8.5343 GiB/s 8.5845 GiB/s 8.6280 GiB/s]
hyperscan/Medium/32768  time:   [1.2145 us 1.2289 us 1.2502 us]
                        thrpt:  [24.410 GiB/s 24.834 GiB/s 25.128 GiB/s]
hyperscan/Medium/1048576
                        time:   [36.109 us 37.559 us 40.516 us]
                        thrpt:  [24.103 GiB/s 26.001 GiB/s 27.045 GiB/s]
hyperscan/Medium/33554432
                        time:   [2.1737 ms 2.1863 ms 2.1998 ms]
                        thrpt:  [14.206 GiB/s 14.293 GiB/s 14.376 GiB/s]
hyperscan/Easy0/16      time:   [7.6203 ns 7.7540 ns 7.9810 ns]
                        thrpt:  [1.8671 GiB/s 1.9217 GiB/s 1.9555 GiB/s]
hyperscan/Easy0/32      time:   [59.472 ns 60.330 ns 61.681 ns]
                        thrpt:  [494.76 MiB/s 505.85 MiB/s 513.14 MiB/s]
hyperscan/Easy0/1024    time:   [100.62 ns 102.37 ns 105.50 ns]
                        thrpt:  [9.0395 GiB/s 9.3161 GiB/s 9.4779 GiB/s]
hyperscan/Easy0/32768   time:   [1.1906 us 1.1943 us 1.1987 us]
                        thrpt:  [25.459 GiB/s 25.552 GiB/s 25.632 GiB/s]
hyperscan/Easy0/1048576 time:   [36.014 us 36.183 us 36.378 us]
                        thrpt:  [26.845 GiB/s 26.990 GiB/s 27.116 GiB/s]
hyperscan/Easy0/33554432
                        time:   [2.1796 ms 2.2412 ms 2.3535 ms]
                        thrpt:  [13.278 GiB/s 13.943 GiB/s 14.338 GiB/s]
hyperscan/Easy1/16      time:   [7.5647 ns 7.5955 ns 7.6288 ns]
                        thrpt:  [1.9533 GiB/s 1.9618 GiB/s 1.9698 GiB/s]
hyperscan/Easy1/32      time:   [78.666 ns 79.158 ns 79.806 ns]
                        thrpt:  [382.40 MiB/s 385.53 MiB/s 387.94 MiB/s]
hyperscan/Easy1/1024    time:   [139.51 ns 139.88 ns 140.28 ns]
                        thrpt:  [6.7982 GiB/s 6.8179 GiB/s 6.8359 GiB/s]
hyperscan/Easy1/32768   time:   [1.7645 us 1.8316 us 1.9747 us]
                        thrpt:  [15.454 GiB/s 16.661 GiB/s 17.296 GiB/s]
hyperscan/Easy1/1048576 time:   [54.465 us 54.690 us 54.957 us]
                        thrpt:  [17.770 GiB/s 17.856 GiB/s 17.930 GiB/s]
hyperscan/Easy1/33554432
                        time:   [2.5835 ms 2.5940 ms 2.6056 ms]
                        thrpt:  [11.994 GiB/s 12.047 GiB/s 12.096 GiB/s]
hyperscan/Easy0i/16     time:   [7.5739 ns 7.5992 ns 7.6260 ns]
                        thrpt:  [1.9540 GiB/s 1.9609 GiB/s 1.9674 GiB/s]
hyperscan/Easy0i/32     time:   [63.005 ns 64.754 ns 67.552 ns]
                        thrpt:  [451.77 MiB/s 471.28 MiB/s 484.37 MiB/s]
hyperscan/Easy0i/1024   time:   [114.28 ns 116.08 ns 118.43 ns]
                        thrpt:  [8.0526 GiB/s 8.2157 GiB/s 8.3450 GiB/s]
hyperscan/Easy0i/32768  time:   [1.3169 us 1.3209 us 1.3256 us]
                        thrpt:  [23.022 GiB/s 23.103 GiB/s 23.174 GiB/s]
hyperscan/Easy0i/1048576
                        time:   [40.118 us 40.363 us 40.628 us]
                        thrpt:  [24.037 GiB/s 24.194 GiB/s 24.342 GiB/s]
hyperscan/Easy0i/33554432
                        time:   [2.2489 ms 2.2769 ms 2.3150 ms]
                        thrpt:  [13.499 GiB/s 13.725 GiB/s 13.896 GiB/s]
hyperscan/Hard1/16      time:   [38.670 ns 40.512 ns 43.454 ns]
                        thrpt:  [351.15 MiB/s 376.65 MiB/s 394.59 MiB/s]
hyperscan/Hard1/32      time:   [40.461 ns 40.692 ns 40.890 ns]
                        thrpt:  [746.32 MiB/s 749.97 MiB/s 754.24 MiB/s]
hyperscan/Hard1/1024    time:   [68.407 ns 68.681 ns 69.002 ns]
                        thrpt:  [13.821 GiB/s 13.886 GiB/s 13.941 GiB/s]
hyperscan/Hard1/32768   time:   [1.0370 us 1.0411 us 1.0455 us]
                        thrpt:  [29.190 GiB/s 29.314 GiB/s 29.429 GiB/s]
hyperscan/Hard1/1048576 time:   [34.754 us 35.279 us 36.310 us]
                        thrpt:  [26.895 GiB/s 27.681 GiB/s 28.099 GiB/s]
hyperscan/Hard1/33554432
                        time:   [2.3153 ms 2.3878 ms 2.4730 ms]
                        thrpt:  [12.637 GiB/s 13.087 GiB/s 13.497 GiB/s]

chimera/Hard/16         time:   [21.959 ns 22.034 ns 22.121 ns]
                        thrpt:  [689.79 MiB/s 692.50 MiB/s 694.87 MiB/s]
chimera/Hard/32         time:   [45.730 ns 45.872 ns 46.046 ns]
                        thrpt:  [662.76 MiB/s 665.28 MiB/s 667.34 MiB/s]
chimera/Hard/1024       time:   [116.41 ns 121.81 ns 132.81 ns]
                        thrpt:  [7.1810 GiB/s 7.8292 GiB/s 8.1923 GiB/s]
chimera/Hard/32768      time:   [1.2021 us 1.2064 us 1.2124 us]
                        thrpt:  [25.171 GiB/s 25.296 GiB/s 25.387 GiB/s]
chimera/Hard/1048576    time:   [35.805 us 35.916 us 36.050 us]
                        thrpt:  [27.089 GiB/s 27.190 GiB/s 27.274 GiB/s]
chimera/Hard/33554432   time:   [2.2012 ms 2.2206 ms 2.2414 ms]
                        thrpt:  [13.942 GiB/s 14.073 GiB/s 14.197 GiB/s]
chimera/Medium/16       time:   [21.899 ns 22.288 ns 23.108 ns]
                        thrpt:  [660.33 MiB/s 684.63 MiB/s 696.77 MiB/s]
chimera/Medium/32       time:   [49.151 ns 49.310 ns 49.487 ns]
                        thrpt:  [616.68 MiB/s 618.89 MiB/s 620.89 MiB/s]
chimera/Medium/1024     time:   [128.00 ns 128.48 ns 129.00 ns]
                        thrpt:  [7.3926 GiB/s 7.4229 GiB/s 7.4505 GiB/s]
chimera/Medium/32768    time:   [1.2143 us 1.2178 us 1.2219 us]
                        thrpt:  [24.976 GiB/s 25.059 GiB/s 25.132 GiB/s]
chimera/Medium/1048576  time:   [35.888 us 36.224 us 36.839 us]
                        thrpt:  [26.509 GiB/s 26.959 GiB/s 27.212 GiB/s]
chimera/Medium/33554432 time:   [2.2379 ms 2.2767 ms 2.3200 ms]
                        thrpt:  [13.470 GiB/s 13.726 GiB/s 13.964 GiB/s]
chimera/Easy0/16        time:   [22.006 ns 22.074 ns 22.145 ns]
                        thrpt:  [689.05 MiB/s 691.25 MiB/s 693.40 MiB/s]
chimera/Easy0/32        time:   [45.707 ns 46.510 ns 47.881 ns]
                        thrpt:  [637.37 MiB/s 656.15 MiB/s 667.68 MiB/s]
chimera/Easy0/1024      time:   [116.18 ns 118.38 ns 121.96 ns]
                        thrpt:  [7.8196 GiB/s 8.0559 GiB/s 8.2085 GiB/s]
chimera/Easy0/32768     time:   [1.2089 us 1.2149 us 1.2222 us]
                        thrpt:  [24.970 GiB/s 25.120 GiB/s 25.245 GiB/s]
chimera/Easy0/1048576   time:   [35.747 us 35.839 us 35.949 us]
                        thrpt:  [27.165 GiB/s 27.248 GiB/s 27.318 GiB/s]
chimera/Easy0/33554432  time:   [2.1717 ms 2.1862 ms 2.2021 ms]
                        thrpt:  [14.191 GiB/s 14.294 GiB/s 14.389 GiB/s]
chimera/Easy1/16        time:   [21.898 ns 21.957 ns 22.019 ns]
                        thrpt:  [692.99 MiB/s 694.95 MiB/s 696.80 MiB/s]
chimera/Easy1/32        time:   [54.226 ns 58.548 ns 63.641 ns]
                        thrpt:  [479.53 MiB/s 521.24 MiB/s 562.78 MiB/s]
chimera/Easy1/1024      time:   [156.46 ns 156.94 ns 157.50 ns]
                        thrpt:  [6.0551 GiB/s 6.0769 GiB/s 6.0954 GiB/s]
chimera/Easy1/32768     time:   [1.9545 us 2.4792 us 3.5112 us]
                        thrpt:  [8.6914 GiB/s 12.310 GiB/s 15.614 GiB/s]
chimera/Easy1/1048576   time:   [61.846 us 80.517 us 107.45 us]
                        thrpt:  [9.0885 GiB/s 12.129 GiB/s 15.790 GiB/s]
chimera/Easy1/33554432  time:   [3.5006 ms 3.8279 ms 4.2312 ms]
                        thrpt:  [7.3857 GiB/s 8.1637 GiB/s 8.9271 GiB/s]
chimera/Easy0i/16       time:   [32.880 ns 37.164 ns 42.523 ns]
                        thrpt:  [358.83 MiB/s 410.58 MiB/s 464.07 MiB/s]
chimera/Easy0i/32       time:   [56.519 ns 60.221 ns 64.878 ns]
                        thrpt:  [470.38 MiB/s 506.76 MiB/s 539.95 MiB/s]
chimera/Easy0i/1024     time:   [131.94 ns 132.87 ns 133.96 ns]
                        thrpt:  [7.1191 GiB/s 7.1775 GiB/s 7.2283 GiB/s]
chimera/Easy0i/32768    time:   [1.6345 us 1.7481 us 1.9095 us]
                        thrpt:  [15.982 GiB/s 17.457 GiB/s 18.671 GiB/s]
chimera/Easy0i/1048576  time:   [45.820 us 47.466 us 49.573 us]
                        thrpt:  [19.699 GiB/s 20.574 GiB/s 21.313 GiB/s]
chimera/Easy0i/33554432 time:   [2.6227 ms 2.6561 ms 2.6926 ms]
                        thrpt:  [11.606 GiB/s 11.765 GiB/s 11.915 GiB/s]
chimera/Hard1/16        time:   [62.354 ns 63.248 ns 64.400 ns]
                        thrpt:  [236.94 MiB/s 241.25 MiB/s 244.71 MiB/s]
chimera/Hard1/32        time:   [62.204 ns 64.787 ns 68.518 ns]
                        thrpt:  [445.39 MiB/s 471.05 MiB/s 490.60 MiB/s]
chimera/Hard1/1024      time:   [100.28 ns 107.50 ns 119.79 ns]
                        thrpt:  [7.9612 GiB/s 8.8711 GiB/s 9.5099 GiB/s]
chimera/Hard1/32768     time:   [1.1941 us 1.2423 us 1.3333 us]
                        thrpt:  [22.889 GiB/s 24.565 GiB/s 25.557 GiB/s]
chimera/Hard1/1048576   time:   [40.332 us 41.404 us 42.798 us]
                        thrpt:  [22.818 GiB/s 23.586 GiB/s 24.213 GiB/s]
chimera/Hard1/33554432  time:   [3.0922 ms 3.4004 ms 3.7717 ms]
                        thrpt:  [8.2854 GiB/s 9.1901 GiB/s 10.106 GiB/s]

regex/Hard/16           time:   [65.931 ns 67.258 ns 68.895 ns]
                        thrpt:  [221.48 MiB/s 226.87 MiB/s 231.44 MiB/s]
regex/Hard/32           time:   [113.10 ns 121.71 ns 132.71 ns]
                        thrpt:  [229.95 MiB/s 250.73 MiB/s 269.82 MiB/s]
regex/Hard/1024         time:   [2.1262 us 2.1694 us 2.2317 us]
                        thrpt:  [437.58 MiB/s 450.16 MiB/s 459.30 MiB/s]
regex/Hard/32768        time:   [68.672 us 74.051 us 81.856 us]
                        thrpt:  [381.77 MiB/s 422.01 MiB/s 455.06 MiB/s]
regex/Hard/1048576      time:   [2.1402 ms 2.1658 ms 2.1936 ms]
                        thrpt:  [455.86 MiB/s 461.73 MiB/s 467.24 MiB/s]
Benchmarking regex/Hard/33554432: Warming up for 3.0000 s
Warning: Unable to complete 100 samples in 5.0s. You may wish to increase target time to 6.8s, or reduce sample count to 70.
regex/Hard/33554432     time:   [69.916 ms 72.581 ms 76.022 ms]
                        thrpt:  [420.93 MiB/s 440.88 MiB/s 457.69 MiB/s]
regex/Medium/16         time:   [39.695 ns 40.709 ns 41.880 ns]
                        thrpt:  [364.34 MiB/s 374.82 MiB/s 384.40 MiB/s]
regex/Medium/32         time:   [51.597 ns 51.816 ns 52.057 ns]
                        thrpt:  [586.23 MiB/s 588.96 MiB/s 591.45 MiB/s]
regex/Medium/1024       time:   [158.24 ns 162.78 ns 169.19 ns]
                        thrpt:  [5.6365 GiB/s 5.8587 GiB/s 6.0269 GiB/s]
regex/Medium/32768      time:   [3.6467 us 3.7579 us 3.9074 us]
                        thrpt:  [7.8102 GiB/s 8.1210 GiB/s 8.3685 GiB/s]
regex/Medium/1048576    time:   [125.20 us 134.81 us 150.08 us]
                        thrpt:  [6.5072 GiB/s 7.2439 GiB/s 7.7998 GiB/s]
regex/Medium/33554432   time:   [4.6222 ms 4.8445 ms 5.1140 ms]
                        thrpt:  [6.1106 GiB/s 6.4507 GiB/s 6.7609 GiB/s]
regex/Easy0/16          time:   [34.249 ns 34.529 ns 34.823 ns]
                        thrpt:  [438.18 MiB/s 441.91 MiB/s 445.52 MiB/s]
regex/Easy0/32          time:   [45.272 ns 45.964 ns 46.787 ns]
                        thrpt:  [652.26 MiB/s 663.95 MiB/s 674.09 MiB/s]
regex/Easy0/1024        time:   [66.692 ns 67.038 ns 67.416 ns]
                        thrpt:  [14.146 GiB/s 14.226 GiB/s 14.300 GiB/s]
regex/Easy0/32768       time:   [1.1983 us 1.2147 us 1.2343 us]
                        thrpt:  [24.725 GiB/s 25.123 GiB/s 25.467 GiB/s]
regex/Easy0/1048576     time:   [43.438 us 44.470 us 45.709 us]
                        thrpt:  [21.365 GiB/s 21.960 GiB/s 22.482 GiB/s]
regex/Easy0/33554432    time:   [2.4127 ms 2.4772 ms 2.5653 ms]
                        thrpt:  [12.182 GiB/s 12.615 GiB/s 12.952 GiB/s]
regex/Easy1/16          time:   [50.337 ns 50.917 ns 51.635 ns]
                        thrpt:  [295.51 MiB/s 299.68 MiB/s 303.14 MiB/s]
regex/Easy1/32          time:   [50.931 ns 52.127 ns 53.770 ns]
                        thrpt:  [567.56 MiB/s 585.45 MiB/s 599.20 MiB/s]
regex/Easy1/1024        time:   [66.875 ns 72.588 ns 79.624 ns]
                        thrpt:  [11.977 GiB/s 13.138 GiB/s 14.261 GiB/s]
regex/Easy1/32768       time:   [448.08 ns 489.86 ns 551.24 ns]
                        thrpt:  [55.361 GiB/s 62.299 GiB/s 68.108 GiB/s]
regex/Easy1/1048576     time:   [25.172 us 28.716 us 33.628 us]
                        thrpt:  [29.040 GiB/s 34.008 GiB/s 38.796 GiB/s]
regex/Easy1/33554432    time:   [2.3208 ms 2.4041 ms 2.4967 ms]
                        thrpt:  [12.517 GiB/s 12.999 GiB/s 13.465 GiB/s]
regex/Easy0i/16         time:   [59.379 ns 60.330 ns 61.471 ns]
                        thrpt:  [248.23 MiB/s 252.92 MiB/s 256.97 MiB/s]
regex/Easy0i/32         time:   [86.053 ns 91.097 ns 98.327 ns]
                        thrpt:  [310.37 MiB/s 335.00 MiB/s 354.64 MiB/s]
regex/Easy0i/1024       time:   [151.25 ns 152.59 ns 154.10 ns]
                        thrpt:  [6.1888 GiB/s 6.2499 GiB/s 6.3052 GiB/s]
regex/Easy0i/32768      time:   [3.6745 us 3.7282 us 3.7843 us]
                        thrpt:  [8.0642 GiB/s 8.1857 GiB/s 8.3052 GiB/s]
regex/Easy0i/1048576    time:   [134.03 us 142.00 us 151.84 us]
                        thrpt:  [6.4317 GiB/s 6.8774 GiB/s 7.2859 GiB/s]
regex/Easy0i/33554432   time:   [4.9993 ms 5.7185 ms 6.6768 ms]
                        thrpt:  [4.6804 GiB/s 5.4647 GiB/s 6.2509 GiB/s]
regex/Hard1/16          time:   [56.829 ns 62.347 ns 71.392 ns]
                        thrpt:  [213.73 MiB/s 244.74 MiB/s 268.51 MiB/s]
regex/Hard1/32          time:   [99.740 ns 109.86 ns 122.72 ns]
                        thrpt:  [248.67 MiB/s 277.78 MiB/s 305.97 MiB/s]
regex/Hard1/1024        time:   [155.81 ns 166.22 ns 181.80 ns]
                        thrpt:  [5.2458 GiB/s 5.7375 GiB/s 6.1209 GiB/s]
regex/Hard1/32768       time:   [4.2781 us 4.5519 us 4.9237 us]
                        thrpt:  [6.1981 GiB/s 6.7044 GiB/s 7.1334 GiB/s]
regex/Hard1/1048576     time:   [199.14 us 242.67 us 293.85 us]
                        thrpt:  [3.3233 GiB/s 4.0243 GiB/s 4.9040 GiB/s]
regex/Hard1/33554432    time:   [6.0598 ms 6.6442 ms 7.3072 ms]
                        thrpt:  [4.2766 GiB/s 4.7034 GiB/s 5.1569 GiB/s]
```

## License

This project is licensed under either of Apache License [APACHE-2.0](https://spdx.org/licenses/Apache-2.0.html) or MIT license [MIT](https://spdx.org/licenses/MIT.html) at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Futures by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
