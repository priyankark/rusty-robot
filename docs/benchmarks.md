# RobotJS comparison — 2026-10-27

This compares **rusty-robot's Rust API with RobotJS's JavaScript API backed by
its native C/C++ addon**. It is not Rust versus a pure JavaScript implementation.
No production implementation was changed to improve these baseline results.
Subsequent algorithm-equivalent optimizations and measurements are recorded in
[optimization results](optimization-results.md); the table below preserves the baseline.

## Findings

Results are mixed. Rust avoids much of the binding and object-conversion cost:
bitmap pixel reads are 15.5× faster, and returning 65,536 color matches is 164×
faster. That bulk-result case compares a Rust `Vec<Point>` with a JavaScript array
of objects; it is not evidence that the underlying search is 164× faster.

RobotJS is faster on exact full-image scans: Rust's late 32×32 image search takes
1.12× as long (2.359 ms versus 2.112 ms), and absent-color counting takes Rust
1.61× as long. Tolerant image search favors
Rust by 1.21×. Small captures are the largest desktop regression: Rust takes 5.52×
as long for 100×100 and 6.14× as long for a single pixel. Full-screen capture is
roughly comparable; its 1.10× median advantage for Rust is within overlapping
ranges across rounds, so it should not be treated as a robust win.

The immediate optimization candidates are region/pixel capture and exact-match
scan paths. Rust currently captures the full native display through
ScreenCaptureKit, then crops; RobotJS requests its rectangle through the legacy
CoreGraphics capture API. Different backends and sampling behavior contribute
to the capture results, so these timings do not isolate language overhead.

## Method

- This Mac: Apple M6, 32 GiB RAM, macOS 27.0, arm64. Node v24.22.0 and Rust 1.99.1.
- RobotJS 0.9.1 at `0af6073e1f1bcff865ffe52801c71647829cdc1c`, built locally with
  node-gyp Release (`-O3`) and PNG enabled. The original reference checkout remains
  unchanged; the build uses a separate local clone under `target/benchmark`.
- Rust uses the normal Cargo release profile (`opt-level=3`), without custom
  `RUSTFLAGS`. Builds and fixture construction are outside measured intervals.
- Each operation runs in a fresh process per round. Five warmup calls precede
  adaptive calibration toward 40 ms per batch, capped at 262,144 calls. Seven
  batches per process, three rounds: the table reports the median of 21 batch
  averages, not individual-call tail latency. The Rust and Node runners consume
  results, with `black_box` used in Rust. API wrapper/loop overhead is included.
- Case order is shuffled deterministically; implementation order is Rust/JS,
  JS/Rust, Rust/JS over the three rounds. Execution is sequential, not concurrent.
  Node garbage collection is left at its default behavior. File I/O uses warmed
  OS caches; saves include encoding/writing but no durability `fsync`.
- Both use identical generated 1920×1080 BGRA data. RGB background channels stay
  below 128; a 32×32 needle is planted at `(1856, 1016)`. Absent needles differ at
  both corners so the two implementations' scan directions are not advantaged by
  a single mismatching corner. Tolerance is 0.02 for the tolerant case.
- Dense inputs are uniform 256×256 images: 65,536 color results, or 64,009
  overlapping 4×4 image results. These deliberately expose result-allocation
  costs. This synthetic corpus does not represent all real screenshots.
- Before timing, runners assert expected search results, dimensions and counts;
  codec results are checked at five known pixels. Comparable observations are
  checked across languages and rounds. Native cursor/pixel values are consumed
  but cannot be equality-checked across time on a live desktop.
- Read-only desktop benchmarks do not type, click, move the pointer or retain
  captured screen images on disk. Captures cover the main 1920×1080 logical
  display; small capture is 100×100 at `(0, 0)`.

## Codec qualifications

The API defaults produce equivalent tested RGB values but different storage:
RobotJS decodes the source BMP/PNG to 24-bit BGR; Rust produces 32-bit BGRA. Thus
Rust's 3.04× slower BMP load includes expansion into a larger buffer. PNG load is
1.18× faster despite that conversion.

BMP saving also differs: RobotJS writes 8,294,454 bytes (32-bit) and Rust writes
6,220,854 bytes (24-bit). PNG defaults take essentially equal time, but Rust's
88,500-byte output is 14.5% larger than RobotJS's 77,311 bytes. The public APIs do
not offer equivalent compression-setting knobs. Treat the codec measurements as
end-to-end default API behavior, not identical encoder configurations.

## Complete results

Median microseconds per operation. Ratio = RobotJS / Rust; above 1 favors Rust.

| Operation | Rust µs | RobotJS µs | Ratio |
| --- | ---: | ---: | ---: |
| color_at | 0.035 | 0.547 | 15.53× |
| find_color_early | 0.014 | 0.665 | 48.18× |
| find_color_late | 1358.756 | 1004.295 | 0.74× |
| find_color_absent | 1467.285 | 1059.801 | 0.72× |
| count_color_absent | 1728.925 | 1070.677 | 0.62× |
| count_color_dense | 50.976 | 115.445 | 2.26× |
| find_colors_dense | 66.433 | 10907.636 | 164.19× |
| find_image_late | 2358.941 | 2111.630 | 0.90× |
| find_image_absent | 2445.022 | 2190.641 | 0.90× |
| find_image_tolerant | 2366.987 | 2859.271 | 1.21× |
| count_image_dense | 1136.410 | 1030.479 | 0.91× |
| find_images_dense | 1107.981 | 11568.615 | 10.44× |
| load_bmp | 1610.712 | 530.432 | 0.33× |
| load_png | 2128.021 | 2519.664 | 1.18× |
| save_bmp | 3252.198 | 2118.742 | 0.65× |
| save_png | 36666.645 | 36667.062 | 1.00× |
| get_mouse_pos | 0.049 | 0.186 | 3.77× |
| get_displays | 18.276 | 14.558 | 0.80× |
| get_pixel_color | 25229.125 | 4112.299 | 0.16× |
| capture_small | 26716.916 | 4841.495 | 0.18× |
| capture_full | 29349.209 | 32200.666 | 1.10× |

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
  "saved_file_bytes": {
    "js-output.bmp": 8294454,
    "rust-output.bmp": 6220854,
    "rust-output.png": 88500,
    "js-output.png": 77311
  },
  "rust_source_sha256": "47b7616d771f16a09c6279b6a00c56fbd98123dca2df1a0e60f21930e03169a9",
  "measured_at_utc": "2026-10-27T15:15:56.618562+00:00"
}
```

Failed measurements: 0. Raw samples: raw.json.

## Reproduce

```sh
python3 scripts/benchmark.py
```

This builds both implementations, creates fixtures, then runs all cases. It
requires Node/npm, the Rust toolchain, native build tools, and libpng available to
`pkg-config` on macOS. The default RobotJS reference is the sibling `../robotjs`
checkout; `--robotjs PATH` selects an already cloned source tree.

For already-built binaries:

```sh
python3 scripts/benchmark.py --skip-build
```

Use `--rounds N` or `--cases NAME ...` for a focused run. Outputs under
`target/benchmark` include every process log, `raw.json` (all samples and fixture
hashes), `summary.csv` (including per-round median ranges), and `summary.md`.
Do not run builds or other benchmarks concurrently with the measurements.
These measurements establish performance on this host only, not on Linux or
Windows and not statistical confidence intervals or long-run application latency.
