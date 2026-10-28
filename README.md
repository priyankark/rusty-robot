<p align="center"><img src="https://raw.githubusercontent.com/priyankark/rusty-robot/main/assets/robot.png" width="180" alt="Rusty Robot: a copper robot with a cursor on its face"></p>
<h1 align="center">Rusty Robot</h1>
<p align="center">Desktop automation in Rust. Built for tools, tests, and agents.</p>
<p align="center">
<a href="https://github.com/priyankark/rusty-robot/actions/workflows/ci.yml"><img src="https://github.com/priyankark/rusty-robot/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
<a href="LICENSE.md"><img src="https://img.shields.io/badge/license-MIT-copper" alt="MIT licensed"></a>
<a href="https://priyankark.github.io/rusty-robot/api/rusty_robot/"><img src="https://img.shields.io/badge/API-Rust-orange" alt="API documentation"></a>
</p>

Move, click, type, capture, and find pixels or images through one Rust API.
Rusty Robot ports the [RobotJS](https://github.com/octalmage/robotjs) test contract
into an independent Rust implementation; it does not require Node.js or a C++ addon.

**0.1 is an early release.** macOS has real desktop verification; Linux/X11 and
Windows pass native desktop CI. Multi-monitor coverage differs by OS; see the dated
[validation record](docs/validation.md) for exactly what has run.

[API documentation](https://priyankark.github.io/rusty-robot/api/rusty_robot/) ·
[Desktop API benchmarks](https://priyankark.github.io/rusty-robot/api-benchmarks.html) ·
[Agent skill](skills/rusty-robot/SKILL.md) · [Contributing](CONTRIBUTING.md)

## Start here

Rust 1.88 or newer. The Cargo package is `rusty-robot-desktop`; the Rust import remains `rusty_robot`:

```toml
[dependencies]
rusty-robot-desktop = "0.1.2"
```

```rust,no_run
use rusty_robot::Robot;

fn main() -> rusty_robot::Result<()> {
    for display in Robot.get_displays()? {
        println!("{display:?}");
    }
    let capture = Robot.capture(None)?; // Main display; preserves scale and origin.
    capture.save(std::path::Path::new("screen.png"))?;
    println!("Top-left RGB: {}", capture.color_at(0, 0)?);
    Ok(())
}
```

Use `Robot.move_mouse(x, y)`, `Robot.mouse_click(Some("left"), false)`,
`Robot.key_tap("a", &["command"])` (macOS) or `&["control"]` (Windows/Linux),
and `Robot.type_string("Hello, world! 🌍")` for input. Input goes to the focused
application. [Recipes](skills/rusty-robot/references/rust-api.md) cover coordinates,
image search, held keys, and verification. Disable the default `png` feature for
BMP-only builds.

## Capabilities and platforms

| Area | Included |
| --- | --- |
| Mouse | Move/smooth move, click/double click, held buttons, drag, two-axis scroll |
| Keyboard | Physical keys, modifier chords, Unicode text, paced typing |
| Screen | Displays, capture, pixel reads, signed origins, Retina/DPI metadata |
| Images | Exact/tolerant color and template search, first/all/count, BMP/PNG |
| Permissions | Query and request Accessibility / Screen Recording on macOS |

| Platform | Backend | Requirements |
| --- | --- | --- |
| macOS 14+ | CoreGraphics + ScreenCaptureKit | Accessibility for input; Screen Recording for capture |
| Linux / X11 | x11rb + XTest | An X11 session with `DISPLAY`; native Wayland unsupported |
| Windows | SendInput + GDI | An interactive desktop; elevated windows may require matching privilege |

Bitmap coordinates are physical pixels; desktop coordinates are logical positions.
Use bitmap conversion methods rather than assuming a scale of one. A capture
rectangle must fit inside a single display. Text input on macOS retains a 10 ms
minimum interval per native text batch (up to 20 UTF-16 units); paced typing
retains per-character intervals. This is based on native delivery testing.

## Measured performance

<!-- optimized-capture:start -->
### Current capture results — faster full-screen automation

**The optimized Rusty Robot beats XCap on repeated full-screen captures on this
Mac. XCap still wins small-region latency.** This is **unreleased source
`1535dc2`**, not the existing crates.io/npm 0.1.2 binaries.

Standalone API comparison on **Apple M6, macOS 27.0, Rust 1.99.1**, against
**XCap 0.9.8**, measured 2026-10-28. No application integrations, network transport,
video encoding, or cached screenshot frames are involved. This is one host,
not a cross-platform ranking. [Enigo + XCap selection and initial baseline](#initial-rust-ecosystem-baseline--before-capture-optimization).

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
XCap returned display-space colors outside that tolerance. A separate 115-capture
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

The [RobotJS API report](https://priyankark.github.io/rusty-robot/api-benchmarks.html) compares **21 operations**
with RobotJS 0.9.1's JavaScript API **and native addon**, on one Apple M6 Mac.
Rust had lower medians in 17 cases, higher in four; this is not a statistical win
count or a universal speedup. Exact color searches were about 2.1× faster;
single-pixel capture took 6.08× as long. Dense result collection includes different
Rust/JavaScript allocation costs. Some round ranges overlap.

[Raw samples and CSV](benchmarks/2026-10-27/) ·
[Methodology](docs/benchmarks.md) · [Optimization history](docs/optimization-results.md)

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

#### Screenshots: XCap is faster on this Mac

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

Rusty Robot's small-region cost is a known limitation: its macOS backend captures
the full display before cropping to avoid partially transparent tiny captures.
XCap uses a direct region capture. This benchmark does not change either algorithm.

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

## For agents

Install the bundled skill into your agent's skill directory, for example:

```sh
cp -R skills/rusty-robot ~/.codex/skills/rusty-robot
```

Read [SKILL.md](skills/rusty-robot/SKILL.md), then use `$rusty-robot` to integrate
native automation or migrate a RobotJS call site. The skill includes tested API
recipes and a guide for Node/TypeScript consumers.

## Development

```sh
cargo test --all-features --locked
cargo test --no-default-features --locked
cargo clippy --all-targets --all-features -- -D warnings
```

The suite includes exhaustive bitmap/codec regressions, compile-fail contracts,
and [77 mapped upstream cases](docs/rust-port.md). Desktop tests are explicitly
opt-in and use owned test windows. See [testing](docs/testing.md),
[release process](docs/releasing.md), and [roadmap](docs/roadmap.md).

## License

[MIT](LICENSE.md), with RobotJS attribution retained in [NOTICE.md](NOTICE.md).

### Composed media workers

The workspace includes `rusty-robot-media` (capture, resize, BLAKE3 frame fingerprints,
JPEG/PNG encoding) and `rusty-robot-node` (Node-API input plus asynchronous media).
Install `@priyankark/rusty-robot@0.1.2` for prebuilt Node-API binaries across seven
desktop targets. No Rust compiler or install scripts are required. See
[Node package requirements and release-archive installation](packages/node/README.md).
The GitHub release archive is usable before its npm mirror is available. For source development,
build with `node scripts/build-node.cjs`.
See [the agent skill](skills/rusty-robot/SKILL.md) for integration and bounded scheduling.

Performance results above cover standalone desktop automation. Media worker and
application integration behavior should be evaluated separately for each consumer.
