# RobotJS-algorithm optimizations — 2026-10-27

Historical results. See the [benchmark overview](../benchmark.md) for current
capture results, metric explanations, and the limits of this older comparison.

This follow-up preserves the [original benchmark](benchmarks.md) and changes only
bitmap search and BMP I/O. RobotJS means its JavaScript API backed by the native
C/C++ addon, not a pure JavaScript implementation.

## Changes

- Exact color scans compare packed RGB values, ignoring alpha, as RobotJS does
  in `rgb.h`. Rust scans validated row slices with fixed-size pixel chunks;
  nonzero tolerance retains its existing squared-distance calculation.
- Image search skips distance arithmetic for exact matches and compares each
  candidate's needle pixels backwards, following `bitmap_find.c`. Candidate
  positions still advance in row-major order and include overlapping matches.
  Comparison direction can help or hurt depending on the image's content.
- BMP loading copies compatible BGRA rows directly and expands BGR rows using
  fixed-size chunks. BMP saving builds one payload and writes it in bulk,
  following `bmp_io.c`, instead of calling the buffered writer per pixel.
  Saving now needs an additional encoded-file-sized buffer (6,220,854 bytes for
  the benchmark). Allocation is fallible and occurs before opening the output.

The public formats remain unchanged: Rust decodes BMP to BGRA32 and saves BMP24.
RobotJS decodes the fixture to BGR24 and saves the benchmark canvas as BMP32, so
codec comparisons still include different storage sizes. PNG and native desktop
backends are unchanged. In particular, small captures still use ScreenCaptureKit
full-display capture plus cropping; replacing that backend is outside this
algorithm-equivalent change and would require renewed pixel-fidelity testing.

## Verification

113 headless tests passed with PNG and 103 without; totals include 31 compile-fail
contracts and overlap between feature configurations. The exhaustive suites
cover every binary 3×3 image and valid color-search rectangle, all binary 2×2
patterns, overlaps, tolerance boundaries, malformed files, and 1,904 codec
roundtrips. Two new regression tests cover distinct RGB channel permutations
with arbitrary alpha/padding and byte-identical BMP serialization across all
four row alignments for both BGR and BGRA sources.

Formatting and warnings-denied Clippy passed on macOS with and without PNG,
and on Linux/Windows cross-targets with PNG. The 36 opt-in native desktop tests
were not rerun for this bitmap-only change; their prior results are recorded in
[validation](validation.md). Linux/Windows native runtime validation remains
outstanding.

## Measurements

Same Apple M6 Mac, toolchains, release settings, fixtures, and method as the
baseline: three rounds, seven adaptive batches per process, sequential execution,
with reversed implementation order in the middle round. File I/O uses warmed OS
caches without `fsync`. These synthetic workloads do not establish a speedup for
every possible image. Timings below are median microseconds per operation.

### Optimized Rust versus RobotJS

Ratio is RobotJS / Rust; values above 1 favor Rust.

| Operation | Rust µs | RobotJS µs | Ratio |
| --- | ---: | ---: | ---: |
| color_at | 0.035 | 0.548 | 15.65× |
| find_color_early | 0.014 | 0.669 | 49.00× |
| find_color_late | 492.551 | 1016.667 | 2.06× |
| find_color_absent | 514.881 | 1070.796 | 2.08× |
| count_color_absent | 635.832 | 1069.923 | 1.68× |
| count_color_dense | 20.790 | 116.672 | 5.61× |
| find_colors_dense | 41.178 | 11015.011 | 267.50× |
| find_image_late | 2089.548 | 2131.626 | 1.02× |
| find_image_absent | 2178.499 | 2201.496 | 1.01× |
| find_image_tolerant | 2445.660 | 2853.878 | 1.17× |
| count_image_dense | 970.204 | 1043.489 | 1.08× |
| find_images_dense | 1014.505 | 11977.073 | 11.81× |
| load_bmp | 604.427 | 536.255 | 0.89× |
| load_png | 2166.530 | 2530.911 | 1.17× |
| save_bmp | 981.205 | 1164.401 | 1.19× |
| save_png | 36640.791 | 37015.062 | 1.01× |
| get_mouse_pos | 0.049 | 0.190 | 3.85× |
| get_displays | 18.790 | 14.761 | 0.79× |
| get_pixel_color | 25212.166 | 4147.753 | 0.16× |
| capture_small | 25949.875 | 4781.971 | 0.18× |
| capture_full | 28133.500 | 31596.646 | 1.12× |

Exact late/absent color search is now about 2× faster than RobotJS. Exact
image-search timings are approximately tied: their round ranges overlap, so
the 1–2% median advantage is not a robust win. BMP load remains about 13% slower
while producing the larger BGRA buffer. BMP save is faster in this run, but
RobotJS varied substantially between rounds (1.080–2.181 ms). Returning dense
match collections also includes Rust Vec versus JavaScript object allocation.

All 126 process measurements completed successfully (882 timed batches); the
runner checked expected results and cross-language observations. Raw data and
per-case logs are under `target/benchmark`; the original measurements and binary
are preserved in `target/benchmark/before-optimization`.

Optimized run metadata:

```json
{
  "platform": "macOS-27.0-arm64-arm-64bit-Mach-O",
  "processor": "arm",
  "node": "v24.22.0",
  "rustc": "rustc 1.99.1 (58cf8f9c3 2026-10-13)",
  "rust_profile": "release (opt-level=3, default Cargo settings)",
  "rustflags": "",
  "robotjs_build": "node-gyp Release, -O3, ROBOTJS_ENABLE_PNG=1",
  "clang": "Apple clang version 21.0.0 (clang-2100.3.34.2)",
  "robotjs_commit": "0af6073e1f1bcff865ffe52801c71647829cdc1c",
  "rounds": 3,
  "samples_per_round": 7,
  "target_batch_ms": 40,
  "fixtures_sha256": {
    "needle.bgra": "7d1950605a4c3d9ff6052b2218fe421e2f094093141c1e9a2934f2c1e4e676e8",
    "source.bmp": "b3e3a2d8e6081948f96f571a463755ae4d2fba70569778886b79563f78ba1571",
    "canvas.bgra": "1e3c83f6da10199c7b18c66691c1bf0b50997e8644e250ba42f581224e201686",
    "source.png": "6e592d09f1be20a69b68ce4a393d0aab250122af2eeb5be2228953a18db44b51"
  },
  "cpu": "Apple M6",
  "memory_bytes": "34359738368",
  "robotjs_decoded_formats": {
    "bmp": {
      "bitsPerPixel": 24,
      "bytesPerPixel": 3,
      "byteWidth": 5760
    },
    "png": {
      "bitsPerPixel": 24,
      "bytesPerPixel": 3,
      "byteWidth": 5760
    }
  },
  "rust_decoded_format": {
    "bitsPerPixel": 32,
    "bytesPerPixel": 4,
    "byteWidth": 7680
  },
  "rust_source_sha256": "a53c1a742dcc39cebe950072296483f04df924f130d4fedabd022a60f330b015",
  "measured_at_utc": "2026-10-27T15:29:39.939014+00:00",
  "saved_file_bytes": {
    "js-output.bmp": 8294454,
    "rust-output.bmp": 6220854,
    "rust-output.png": 88500,
    "js-output.png": 77311
  }
}
```

### Paired comparison with pre-change Rust

The saved baseline executable and optimized executable were also measured in a
separate sequential A/B run: three rounds, seven batches each, shuffled cases,
with before/after order reversed in the middle round. All 78 processes passed
their correctness assertions (546 timed batches); observed results agreed.
This avoids attributing differences between the earlier and current machine
conditions to the implementation. Ratio is before / after.

| Operation | Before µs | After µs | Speedup |
| --- | ---: | ---: | ---: |
| count_color_absent | 1732.033 | 636.490 | 2.72× |
| count_color_dense | 51.179 | 20.443 | 2.50× |
| count_image_dense | 1141.223 | 966.815 | 1.18× |
| find_color_absent | 1477.225 | 515.551 | 2.87× |
| find_color_early | 0.013 | 0.014 | 0.99× |
| find_color_late | 1381.361 | 490.784 | 2.81× |
| find_colors_dense | 66.234 | 41.208 | 1.61× |
| find_image_absent | 2438.624 | 2163.583 | 1.13× |
| find_image_late | 2370.324 | 2097.707 | 1.13× |
| find_image_tolerant | 2370.676 | 2434.516 | 0.97× |
| find_images_dense | 1109.199 | 998.424 | 1.11× |
| load_bmp | 1603.259 | 600.716 | 2.67× |
| save_bmp | 3218.599 | 983.809 | 3.27× |

The changed paths improve 1.11–3.27× on these workloads, except tolerant image
search, which is about 2.7% slower than before; early color matches are essentially
unchanged. The backwards comparison and exact-match branch are therefore not a
universal improvement. Tolerant search remains faster than RobotJS in the full
comparison. BMP saving trades additional memory for fewer writer calls.

Local A/B artifacts: `target/benchmark/optimization-ab.json`,
`target/benchmark/optimization-ab.log`, and
`target/benchmark/compare-before-after.py`. The JSON records executable SHA-256
hashes. Reproduce the full current Rust/RobotJS comparison with:

```sh
python3 scripts/benchmark.py
```

The runner replaces `target/benchmark/raw.json` and summaries on each run;
copy them elsewhere first when preserving a new baseline.
