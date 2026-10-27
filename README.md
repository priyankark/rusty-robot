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
[Interactive benchmarks](https://priyankark.github.io/rusty-robot/) ·
[Agent skill](skills/rusty-robot/SKILL.md) · [Contributing](CONTRIBUTING.md)

## Start here

Rust 1.88 or newer. Until a crates.io release is available, pin the GitHub tag:

```toml
[dependencies]
rusty-robot = { git = "https://github.com/priyankark/rusty-robot", tag = "v0.1.0" }
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

The [full report](https://priyankark.github.io/rusty-robot/) compares **21 operations**
with RobotJS 0.9.1's JavaScript API **and native addon**, on one Apple M6 Mac.
Rust had lower medians in 17 cases, higher in four; this is not a statistical win
count or a universal speedup. Exact color searches were about 2.1× faster;
single-pixel capture took 6.08× as long. Dense result collection includes different
Rust/JavaScript allocation costs. Some round ranges overlap.

[Raw samples and CSV](benchmarks/2026-10-27/) ·
[Methodology](docs/benchmarks.md) · [Optimization history](docs/optimization-results.md)

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
Build the shared addon with `node scripts/build-node.cjs`; its output is
`target/rusty-robot.node`. These companion crates currently ship from Git.
See [the agent skill](skills/rusty-robot/SKILL.md) for integration and bounded scheduling.

The [real application benchmark report](https://priyankark.github.io/rusty-robot/)
separates verified input, complete video delivery and isolated image processing.
The [earlier 21-operation API comparison](https://priyankark.github.io/rusty-robot/api-benchmarks.html)
remains available, including slower Rust cases.
