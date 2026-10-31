# Desktop automation benchmarks

These benchmarks measure how quickly Rusty Robot captures the desktop, reads and
searches pixels, handles image files, and delivers keyboard/mouse input. They are
library tests and a controlled local application task on one Mac. They do not
measure network or remote-desktop latency.

The latest capture implementation takes **22.31 ms** for a full-screen RGB image,
compared with **29.02 ms** for XCap. XCap is still faster for small regions.
The optimized implementation was measured at **source `1535dc2` and is included
in crate 0.1.3**; version 0.1.2 does not include it. Results were recorded on 2026-10-28.

- [Fresh capture and complete interactions](#fresh-capture-and-complete-interactions-2026-10-31-utc)
- [What the measurements mean](#what-the-measurements-mean)
- [Current capture results](#current-capture-results)
- [Historical input and capture baseline](#historical-input-and-capture-baseline)
- [Real-app analysis versus RobotJS](#real-app-image-analysis-versus-robotjs-2026-10-30)
- [RobotJS comparison](#robotjs-comparison-2026-10-27)
- [Maintaining this report](#maintaining-this-report)

<!-- full-loop:start -->
## Fresh capture and complete interactions (2026-10-31 UTC)

We tested the current implementation in a controlled local Chrome approval app.
Rust captures the screen, finds a button, sends native input and captures again
until the app displays the response. The app independently verifies a trusted
click on the right button. This is a local desktop automation task.

**At the normal 10 ms mouse delay, Rust completed 120/120 measured
interactions: median 53.24 ms, p95 58.67 ms.** At zero requested delay,
96/120 attempts failed to produce a verified response within one second.

| Task | Completed | Median | p95 | Failed |
| --- | --- | --- | --- | --- |
| One pixel | 120/120 | 11.33 ms | 13.18 ms | 0 |
| 100 × 100 region | 120/120 | 11.88 ms | 13.00 ms | 0 |
| 1200 × 800 window | 120/120 | 14.17 ms | 18.39 ms | 0 |
| Complete loop, 0 ms mouse delay | 24/120 | 47.10 ms | 51.82 ms | 96 |
| Complete loop, 10 ms mouse delay | 120/120 | 53.24 ms | 58.67 ms | 0 |

Capture rows time the capture call. Loop rows include initial capture, freshness
verification, exact target search, coordinate conversion, mouse movement/click,
and fresh screenshot polling until the changed marker appears. Timings describe
successful attempts only; failures remain in the raw data. p95 is the nearest-rank
95th percentile. Six fresh processes each run five excluded warmups and twenty
measured attempts. Resets, two animation frames plus 50 ms settling, ownership
checks, host IPC and independent DOM receipts are outside timing. Both drivers
check exact grayscale states, a unique 16-bit black-and-white trial ID
(except the one-pixel case), and the exact magenta target.

**RobotJS failed all five preflight captures in each of the five cases.** Remaining
attempts were skipped; there is no current comparative latency or speed ratio.
Permissions passed. A clean rebuild of the pinned addon also failed. A standalone
C capture succeeded; a minimal Node addon calling the same legacy API failed.
That narrows the context without establishing the underlying cause. The older
successful RobotJS measurements are not substituted for missing current samples.

The 49× real-app analysis win above concerns preloaded pixels and many returned
matches. This complete interaction spends most of its time capturing, delivering
input and waiting for the app to render. It does not establish a general Rust
automation speedup. Hardware: Apple M6, macOS 27.0, Node v24.22.0; Rust release.

[Full report and stage chart](https://priyankark.github.io/rusty-robot/building-rusty-robot.html#capture)
· [Every attempt](docs/workflows/2026-10-31-loop/manifest.json)
· [Summary](docs/workflows/2026-10-31-loop/summary.json)
· [Diagnostics](docs/workflows/2026-10-31-loop/diagnostics.json)
· [Reproduction](tools/benchmark-demo/README.md#fresh-capture-and-the-complete-interaction-loop)
<!-- full-loop:end -->

## What the measurements mean

| Metric or term | Plain-language meaning |
| --- | --- |
| Milliseconds (ms) | Time taken; 1,000 ms is one second. Lower is better. |
| Microseconds (µs) | Used for the older RobotJS tests; 1,000 µs is one ms. |
| Median | The middle measured time: a useful description of a typical call. |
| p95 | 95% of successful measured calls finished within this time. It shows slower calls that a median hides. |
| Warm capture | A repeated capture after two startup calls were discarded. |
| First call | The first capture in a fresh process. Includes capture/render initialization, but excludes launching the process and finding the monitor. It is not a reboot-cold measurement. |
| API-only time | Time inside the capture call. Output sizes differ between libraries, so this alone is not the main comparison. |
| Same-size RGB time | Time until a usable, owned image is in CPU memory at matching dimensions, including resizing/repacking. Excludes file encoding and network transport. |
| Logical vs native pixels | On this Retina display, one desktop coordinate spans two native pixels in each direction. XCap's 3840×2160 image is reduced to 1920×1080 for comparison. |
| Round-median range | Lowest and highest median across separate runs. Describes variation; it is not a confidence interval. |
| Speedup | Other time divided by Rust time. 1.30× means Rust takes about 23% less time, not 30% less time. |
| Freshness | The image contains the test patch's latest red/green state, showing the library returned a new frame. |
| Color check | Compare known patch colors with returned RGB channels. sRGB and display-space RGB can represent the same appearance with different numbers. |
| Successful input delivery | The target app received the exact text, click count, or pointer position requested. An API returning successfully is not enough. |

Failed captures or input deliveries are counted separately and excluded from
latency statistics. We do not turn incorrect results into speedups.

## Current capture results

<!-- optimized-capture:start -->
### Repeated captures

**The optimized Rusty Robot beats XCap on repeated full-screen captures on this
Mac. XCap still wins small-region latency.** This is **source `1535dc2`, included in crate 0.1.3**. The measurements were
made before release; existing crates.io/npm 0.1.2 binaries do not include this change.

Standalone API comparison on **Apple M6, macOS 27.0, Rust 1.99.1**, against
**XCap 0.9.8**, measured 2026-10-28. No application integrations, network transport,
video encoding, or cached screenshot frames are involved. This is one host,
not a cross-platform ranking. [Enigo + XCap selection and initial baseline](#historical-input-and-capture-baseline).

Each result is an owned CPU RGB buffer at the same requested logical dimensions;
XCap's native 2× image is center-sampled down to match. Rusty Robot also converts
to **sRGB**; XCap's values remain in display color space. Thus dimensions match,
but color processing does not: XCap's numbers exclude sRGB conversion.

| Capture | Previous Rust (ms) | Optimized Rust (ms) | XCap (ms) | Improvement over previous Rust | Current comparison |
| --- | ---: | ---: | ---: | ---: | --- |
| Full display, 1920×1080 | 50.98 | 22.31 | 29.02 | 2.29× | Rust 1.30× faster |
| 100×100 region | 44.20 | 9.57 | 8.22 | 4.62× | XCap 1.16× faster |
| One pixel | 45.37 | 9.54 | 8.24 | 4.75× | XCap 1.16× faster |

These are medians of **90 measured calls per case/library**, from six fresh
processes each. Every library passed **270/270 measured freshness checks**.
The previous binary is byte-identical to the original benchmark binary, verified
by SHA-256. It was rerun alongside both contenders; improvements above are not
calculated across different experiments.

| Capture | Rust API median / p95 (ms) | XCap API median / p95 (ms) | Rust same-size RGB p95 (ms) | XCap same-size RGB p95 (ms) |
| --- | ---: | ---: | ---: | ---: |
| Full display | 21.02 / 23.75 | 27.87 / 30.14 | 25.03 | 31.30 |
| 100×100 | 9.57 / 11.93 | 8.21 / 9.95 | 11.93 | 9.96 |
| One pixel | 9.54 / 11.54 | 8.24 / 9.88 | 11.54 | 9.88 |

| Capture | Rust round-median range, same-size RGB (ms) | XCap round-median range (ms) |
| --- | ---: | ---: |
| Full display | 19.94–23.39 | 25.02–29.60 |
| 100×100 | 9.31–9.76 | 8.10–8.64 |
| One pixel | 9.22–9.82 | 8.17–8.31 |

**Cold calls matter too.** The first full-display API call in each fresh process
includes first-use capture/render initialization, but excludes process startup
and XCap monitor discovery. These six samples are separate from the warm table:

| First API call | Median (ms) | Range (ms) |
| --- | ---: | ---: |
| Previous Rust | 106.70 | 96.19–110.68 |
| Optimized Rust | 43.43 | 42.29–52.66 |
| XCap | 42.82 | 36.20–49.53 |

The cold Rust/XCap ranges overlap: there is no demonstrated cold-call win.

**What changed:** on macOS 15.2+, screenshots use ScreenCaptureKit's direct region
API. This avoids enumerating shareable windows and copying the full display for
tiny requests. One thread-safe Core Image context reuses compiled rendering state
for sRGB conversion and nearest-neighbor reduction; each call captures fresh pixels
and waits for rendering to finish. Render errors propagate. The macOS 14 path,
permission checks, logical-size API contract, cursor exclusion, and coordinate
metadata remain in place. No deprecated capture API was introduced.

**Quality verification:** the measured red/green fixture matched expected sRGB
within ±3/channel for 270/270 optimized captures and 270/270 previous captures;
XCap returned display-space colors outside that tolerance (0/270 matching the sRGB check). All 270 were valid, fresh captures; this is a color-space difference, not a capture failure. A separate 115-capture
check covers 12 palette colors, a one-logical-pixel checkerboard, opaque tiny
captures, display corners, and 900 pixels under the pointer. Full-display and
tiny-region captures agree exactly at the checked fixture points. The broader
palette differs from the old implementation by at most 2/channel; some colors in
both implementations differ from the fixture's nominal RGB by more than 3 because
of display color conversion. This does **not** establish perfect color accuracy,
HDR quality, or universal bit-for-bit equivalence with old captures.

Five rendering regressions additionally check arbitrary sRGB pixels, orientation,
exact center sampling, alpha, native-density rendering internally, invalid sizes,
and concurrent calls. The public capture API still returns logical-resolution
images on macOS; this update does not add a native-resolution public API.

**Protocol:** two warmups, then 15 measured calls per case/process; six rounds
rotate the three libraries through every execution position twice. Each capture
follows a red/green fixture change and a 40 ms presentation wait outside timing.
The timer ends at an owned CPU buffer. Cases are full display, 100×100, then one
pixel; p95 is nearest rank. Round ranges describe variation, not confidence
intervals. Existing desktop processes remain running. No energy-use, Linux,
Windows, Intel Mac, secondary-display, or persistent-video-stream speed claim is
made. Full input timing remains pending an idle desktop; the initial pilot below
is not upgraded to a larger result by these capture tests.

Reproduce from an unlocked macOS desktop with Screen Recording permission:

```sh
cargo build --release --locked --manifest-path tools/rust-ecosystem-benchmark/Cargo.toml
python3 tools/rust-ecosystem-benchmark/run.py /tmp/capture-new --capture-only --rounds 6
# Add --baseline /path/to/old/driver for the balanced three-library experiment.
python3 tools/rust-ecosystem-benchmark/verify_capture.py /tmp/capture-quality.json
# Optional --cursor also needs Accessibility; it moves/restores only the pointer.
python3 tools/rust-ecosystem-benchmark/optimized_report.py --check
```

For the historical driver, build `tools/rust-ecosystem-benchmark` at `caff367`
in a separate checkout; its core is the original `457a095` source.
The exact binaries measured here are identified in the manifest.

[Raw samples](benchmarks/2026-10-28-capture-optimized/samples.jsonl) ·
[All summaries](benchmarks/2026-10-28-capture-optimized/summary.json) ·
[Environment and hashes](benchmarks/2026-10-28-capture-optimized/manifest.json) ·
[Live quality evidence](benchmarks/2026-10-28-capture-optimized/quality.json) ·
[Scope and omissions](benchmarks/2026-10-28-capture-optimized/notes.json)

<!-- optimized-capture:end -->

## Historical input and capture baseline

The initial capture timings below predate the optimization and are retained for
traceability. Use the current paired experiment above for performance decisions.
The input pilot is separate and has not been expanded.

<!-- rust-ecosystem-benchmark:start -->
<details>
<summary>Initial benchmark and input pilot (2026-10-28)</summary>

### Initial Rust ecosystem baseline — before capture optimization

**Enigo + XCap is our reference Rust stack, not a universal “best” claim.**
[Enigo 0.6.1](https://github.com/enigo-rs/enigo) supplies keyboard/mouse input;
[XCap 0.9.8](https://github.com/nashaofu/xcap) supplies screenshots. Their
crates.io recent-download counts at selection were approximately 1.69M and 1.32M,
versus 212K for rdev and 5K for AutoPilot. Adoption motivated the selection; it
is not evidence of speed or reliability. [Selection snapshot](benchmarks/2026-10-28-rust-ecosystem/selection.json).

This compares **standalone desktop automation APIs**, without application
integrations, JavaScript bridges, network transport, or video encoding.
Rusty Robot is source revision `457a095` (package version 0.1.2); competitor
versions and transitive dependencies are pinned in the benchmark Cargo.lock.

#### Historical screenshots: before the optimization

All times below are milliseconds; **lower is better**. Each cell is **median / p95**.
“Same-size RGB” includes converting each result to packed RGB at the requested
logical dimensions, using center-sampled nearest-neighbor reduction for XCap's
2× Retina image. This equalizes dimensions, **not color space**.

| Capture | Rusty Robot API | XCap API | Rusty Robot same-size RGB | XCap same-size RGB | XCap median advantage |
| --- | ---: | ---: | ---: | ---: | ---: |
| Full display, 1920×1080 logical | 46.56 / 51.25 | 25.49 / 29.47 | 47.84 / 52.54 | 26.66 / 30.70 | 1.79× |
| 100×100 logical region | 43.53 / 47.78 | 8.23 / 9.53 | 43.54 / 47.79 | 8.24 / 9.54 | 5.28× |
| One logical pixel | 43.08 / 46.72 | 8.23 / 10.14 | 43.08 / 46.72 | 8.23 / 10.14 | 5.23× |

Independent-round median ranges for the same-size RGB workload:

| Capture | Rusty Robot range (ms) | XCap range (ms) |
| --- | ---: | ---: |
| Full display | 34.98–49.76 | 23.70–27.24 |
| 100×100 region | 29.79–46.42 | 7.92–8.70 |
| One pixel | 29.97–44.18 | 8.17–8.46 |

Both libraries returned correctly sized, fresh red/green fixture content in
**135/135 measured captures**. Color behavior differs: Rusty Robot matched the
fixture's sRGB primaries within ±3/channel in 135/135; XCap did so in 0/135.
For example, the red `[255, 0, 0]` patch was returned by XCap as `[235, 47, 25]`,
and green `[0, 255, 0]` as `[93, 250, 25]` on this display. Those are display-space
values, not missing screenshots. XCap's macOS path returns raw CGImage RGB bytes
without an attached color profile; Rusty Robot explicitly requests sRGB. No ICC
conversion cost is included for XCap, so this is **not an equal-color-fidelity
speed comparison**. That matters for automation using exact RGB matches.

The previous Rusty Robot implementation captured
the full display before cropping to avoid partially transparent tiny captures.
XCap uses a direct region capture. The current capture implementation above removes this full-display capture cost on macOS 15.2+.

#### Keyboard and mouse: pilot only; full timing run pending

An owned AppKit editor/button pilot exercised ASCII, mixed Unicode, 32 emoji,
multiline text, a leading newline, select-all/replace, Return, select-all/delete,
move-and-click, and pointer movement. Results are exact observed text, click
counts, or cursor coordinates—not successful API returns.

| Input configuration | Exact successful deliveries | Measured trials per case |
| --- | ---: | ---: |
| Rusty Robot default 10 ms configured delays | 20/20 | 2 |
| Rusty Robot configured delays set to zero | 20/20 | 2 |
| Enigo defaults | 18/20 | 2 |

Enigo inserted `U+200B` before the leading newline in both measured trials
(`"\nsecond line"` became `"\u200b\nsecond line"`). All its other pilot cases
passed. Rusty Robot retains a minimum 10 ms interval per text batch even when
configured delays are zero; Enigo's macOS text API does not sleep between batches.
The pilot is too small to rank input latency or general reliability. Two larger
input attempts stopped when the owned window lost focus; **neither contributes
to published results**. A full run needs an idle desktop. No failed delivery is
assigned a speedup. [Unchanged pilot evidence](benchmarks/2026-10-28-rust-ecosystem/input-pilot/).

#### Methodology and reproduction

- Apple M6, 32 GiB RAM, macOS 27.0 build 26A425, Rust 1.99.1, release build;
  one main display at 1920×1080 logical / 3840×2160 captured pixels for XCap.
- Capture: three fresh processes per library, alternating backend order;
  two warmups then 15 measured calls per case/process (45 per table row/backend).
  Full display, 100×100, and one-pixel cases run in that order. Capture selection
  is warmed: XCap's monitor is cached as its API intends; Rusty Robot uses its
  normal public capture call. Startup and monitor enumeration are not included.
- The owned window toggles a red/green patch before each capture, waits 40 ms
  outside the timer, and checks the returned patch, dimensions, and scale.
  The capture timer ends when an owned CPU buffer is available; no image files
  are encoded or saved. No private screenshots are retained.
- Input pilot: two warmups and two measured trials per case/profile. Fixture
  reset/focus occurs outside timing; measured input goes through native OS APIs.
  State is polled every 2 ms with a 750 ms timeout and checked again 20 ms after
  success for stability. Raw observed latency includes local control IPC and
  polling, not just OS dispatch. The run stops if its owned window loses focus.
- Medians/p95 use measured successful samples only; round ranges are descriptive,
  not confidence intervals. Background desktop processes were left running.
  The visible Rusty Robot variation is retained rather than selecting its fastest
  round. No cross-platform, Chromium, persistent-video-stream, image-search,
  power-use, or universal performance claim follows from this run. Enigo/XCap
  do not provide a matching bitmap-search API for that part of Rusty Robot.

```sh
cargo build --release --locked --manifest-path tools/rust-ecosystem-benchmark/Cargo.toml
# Requires Screen Recording and a visible desktop. No injected input:
python3 tools/rust-ecosystem-benchmark/run.py /tmp/rust-capture --capture-only
# Requires Accessibility too; leave keyboard/mouse idle with the fixture focused:
python3 tools/rust-ecosystem-benchmark/run.py /tmp/rust-full
python3 -m unittest discover -s tools/rust-ecosystem-benchmark -p 'test_*.py'
python3 tools/rust-ecosystem-benchmark/report.py --check
```

[Raw samples](benchmarks/2026-10-28-rust-ecosystem/samples.jsonl) ·
[Summary with all round medians](benchmarks/2026-10-28-rust-ecosystem/summary.json) ·
[Environment and binary hashes](benchmarks/2026-10-28-rust-ecosystem/manifest.json) ·
[Benchmark source and lockfile](tools/rust-ecosystem-benchmark/)

</details>
<!-- rust-ecosystem-benchmark:end -->

## Real-app image analysis versus RobotJS (2026-10-30)

These cases use native Rust captures of actual VS Code and GitHub windows.
Each time covers analysis of **60 preloaded 1200×800 BGRA images** through the
public APIs. Capture, navigation, loading, warmup, and verification are excluded.
Both implementations scan the same rectangle for known theme colors and use the
same row-extents grouping steps. Adjacent highlighted rows share a rectangle.

| Task | RobotJS median | Rust median | Speedup | Verified regions |
| --- | ---: | ---: | ---: | ---: |
| Locate added-code regions in GitHub PR #24 | 3,805.26 ms | 77.39 ms | 49.17× | 64 |
| Locate search highlights in VS Code | 45.00 ms | 14.28 ms | 3.15× | 106 |

Six measured rounds alternate library order. Every answer matches an independent
raw-pixel reference, cross-checked against app layout geometry. All six runs per
library passed in both cases. Separate side-by-side recordings include progress
and recording overhead; their clocks differ from these medians.

The large diff gain includes creating and grouping many matching pixel positions:
RobotJS materializes JavaScript objects; Rust uses a vector. These results do not
measure app responsiveness or compare every possible optimized JavaScript buffer
scan. The colors and search areas are specific to the captured layouts. The
machine is an Apple M6 running macOS 27.0, Node 24.22.0 and Rust 1.99.1; RobotJS
uses the pinned native addon revision recorded in the manifests.

[Watch both app demos and read the story](https://priyankark.github.io/rusty-robot/building-rusty-robot.html#workloads) ·
[GitHub raw evidence](docs/workflows/2026-10-30-github/) ·
[VS Code raw evidence](docs/workflows/2026-10-30-vscode/) ·
[Reproduction and oracle details](tools/benchmark-demo/README.md#real-application-screens).
Each evidence folder includes all trial outputs, native capture provenance,
video, and a ZIP with the exact PNGs and BGRA inputs.

## RobotJS comparison (2026-10-27)

This older experiment compares Rust with the **RobotJS JavaScript API backed by
its native C/C++ addon**. It measured 21 operations, using three rounds and seven
adaptive timing batches per process. Its medians are **batch averages in µs**,
not individual-call p95 latencies. Rust had lower medians in 17 of 21 operations.
This is not an overall application speedup.

| Operation family | What was measured | Result in that experiment |
| --- | --- | --- |
| Pixel read (`color_at`) | Read RGB from an image already in memory. | Rust 15.65× faster. |
| Color search (`find_color_early/late/absent`) | Find the first matching color near the start/end, or scan an image with no match. | Rust 49.00× / 2.06× / 2.08× faster. |
| Color collections (`count_color_absent/dense`, `find_colors_dense`) | Count matches or return all positions; dense means many matches. | Rust 1.68× / 5.61× / 267.50× faster. Returning positions includes language-specific allocation costs. |
| Image search (`find_image_late/absent/tolerant`) | Locate a small template, including approximate colors for the tolerant case. | Rust 1.02× / 1.01× / 1.17× faster. The 1–2% exact-search differences are within overlapping round ranges. |
| Image collections (`count_image_dense`, `find_images_dense`) | Count many template matches or return their positions. | Rust 1.08× / 11.81× faster. |
| Load images (`load_bmp`, `load_png`) | Read and decode an image file into memory. | Rust BMP load about 13% slower; PNG load 1.17× faster. BMP output formats differ. |
| Save images (`save_bmp`, `save_png`) | Encode and write an image, without forcing it to disk with `fsync`. | Rust 1.19× / 1.01× faster; output sizes and codecs differ. |
| Desktop queries (`get_mouse_pos`, `get_displays`) | Query pointer coordinates or display information. | Rust pointer query 3.85× faster; display query about 27% slower. |
| Desktop capture (`get_pixel_color`, `capture_small`, `capture_full`) | Capture one pixel, a small region, or the full display. | Previous Rust was slower for pixel/small capture and 1.12× faster for full capture. These predate the new capture implementation. |

**The latest capture optimization has not been re-benchmarked against RobotJS.**
Do not combine the old RobotJS times with the new XCap experiment to infer a
current RobotJS speedup. The tests also differ in image formats, allocation
behavior, and OS conditions; details are preserved in the dated reports.

[All 21 timings and paired before/after results](docs/optimization-results.md) ·
[Original methods](docs/benchmarks.md) ·
[Interactive historical report](https://priyankark.github.io/rusty-robot/api-benchmarks.html) ·
[Archived samples](benchmarks/2026-10-27/)

## Maintaining this report

The current and historical Rust comparison tables are generated from checked-in
raw samples. To regenerate them and the short README summary:

```sh
python3 tools/rust-ecosystem-benchmark/report.py
python3 tools/rust-ecosystem-benchmark/optimized_report.py
```

CI runs both commands with `--check` and fails if the reports drift from the
recorded evidence. New measurements need a new dated evidence directory; never
replace historical samples to make a result look better.
