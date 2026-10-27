# Testing rusty-robot

## Headless contracts

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings
cargo test --all-features --locked --no-fail-fast
cargo test --no-default-features --locked --no-fail-fast
```

These commands exercise bitmap search, file codecs, metadata validation,
coordinate conversion, image-click effects, permissions, and compile-time API
contracts. Desktop tests are listed as ignored and must be run separately.
`tests/robustness.rs` adds boundary coverage for malformed BMPs, top-down rows,
arithmetic overflow, invalid input without effects, and negative rounding.

The regression suite also runs automatically in the existing CI matrix:

| Location | Coverage |
| --- | --- |
| `src/keyboard/tests.rs` | Fault injection at every chord press/release, lookup failures, cleanup flags, error propagation, Return events, Unicode scalars, and the macOS text pacing floor |
| `tests/search_regressions.rs` | 102,400 color/rectangle cases and 24,576 image-pattern/region cases; first/all/count consistency, overlapping matches, padding, alpha independence, tolerance boundaries, signed coordinate rounding, and failed click output |
| `tests/codec_regressions.rs` | 952 BMP and 952 PNG roundtrips across dimensions, BGR/BGRA and row padding; truncation, 432 BMP header bit mutations, corrupt PNG CRCs, 32-bit BMP alpha, 16-bit PNG channels, one-bit palettes, and preserving existing files on invalid saves |

These tests are deterministic and require no desktop permissions. Keyboard tests
use a private event transport to inject failures; production still uses the native
backend. They verify sequencing and cleanup, not OS delivery. The separate desktop
tests remain necessary for real application behavior. PNG-only cases are excluded
when that feature is disabled.

For a focused regression run:

```sh
cargo test --lib --test search_regressions --test codec_regressions
```

## Real desktop tests

For a recorded, sequential validation run on this Mac:

```sh
python3 scripts/validate-desktop.py --secondary
```

This runs the original desktop cases, primary and negative-origin visual
workflows, and a separate native AppKit editor. Results, timings, explicit
omissions and per-suite exit codes go to `target/desktop-validation/report.json`;
logs and screenshots are saved beside it. A successful process that executes zero
tests or leaves cases ignored is treated as a failure. Use `--dry-run` to inspect
the commands, or `--output PATH` to preserve another run.

The default command without `--secondary` works on Windows and Linux/X11 too,
but explicitly omits the secondary-display cases. The manually dispatched
`.github/workflows/desktop.yml` prepares a Linux Xvfb/Openbox session and a Windows
runner, runs this command, and uploads the evidence even on failures. These jobs
are configured, not yet remotely executed. Their omitted multi-monitor checks
still require suitable hosts. Native macOS validation needs local permission
grants and is run outside the hosted workflow.

Use a desktop session with macOS Accessibility and Screen Recording grants, or
the corresponding Windows/X11 desktop. Rust controls the real pointer and keyboard.
Input smoke tests run inside a disposable Target Practice window. A mutex prevents
tests from injecting concurrent input; cursor and process cleanup run on failures.
Keep the desktop available while the suite runs.

```sh
npm ci --prefix tests/gui
cargo test --test desktop -- --ignored --test-threads=1
```

The fixture needs Node.js and Electron. If npm reports that Electron's install
script was deferred, install its runtime explicitly:

```sh
node tests/gui/node_modules/electron/install.js
```

The secondary-display case requires two active displays. On a single display,
run the other cases explicitly and report the omission:

```sh
cargo test --test desktop -- --ignored --test-threads=1 --skip captures_secondary_display_at_signed_origin
```

On macOS, the optional test-only helper creates a temporary OS virtual monitor
at `(-1280, 0)`, executes a command, and releases the monitor when it finishes:

```sh
cargo run --example virtual_display -- cargo test --test desktop -- --ignored --test-threads=1
```

This helper uses private CoreGraphics virtual-display interfaces, a technique also
used by [Chromium's display tests](https://chromium.googlesource.com/chromium/src/+/HEAD/ui/display/mac/test/virtual_display_util_mac.mm).
It is not part of the library or ordinary test runs. Display arrangement changes
are session-only; the helper checks API availability and reports failures. It can
stop working on future macOS versions. Actual monitor hardware remains useful for
release validation, including mixed scaling and display disconnects.

The fixture waits for the marker center and corners to settle before interacting,
activates its own macOS process across desktop transitions,
and terminates Electron gracefully between cases. Assertions remain in Rust;
there is no RobotJS native addon dependency.

## Visual workflow verification

The additional owned Electron window exercises eight workflows using native
Rust input. Chromium records trusted input events and actual text, click counts,
scroll positions, and drop outcomes. The fixture never synthesizes input through
JavaScript. Each checkpoint saves observed JSON and a Rust BMP capture; on macOS
it also saves an independent `/usr/sbin/screencapture` PNG for visual inspection.

```sh
cargo test --test visual_desktop -- --ignored --nocapture
RUSTY_ROBOT_QA_SECONDARY=1 cargo run --example virtual_display -- cargo test --test visual_desktop -- --ignored --nocapture
```

Artifacts are in `target/desktop-qa/primary` and `target/desktop-qa/secondary`.
Set `RUSTY_ROBOT_QA_ARTIFACTS` to preserve separate runs. The final checkpoint is
`08-all-workflows-passed`; failures save a `failure` checkpoint before cleanup.
Run GUI suites sequentially because they share the real desktop.

Coverage includes single/double/right/middle clicks, smooth movement, held-button
state and drag/drop, both scroll axes and reversal, multilingual text, emoji,
combining characters, multiline input through all three typing APIs, selection,
held Shift and undo, image-search-and-click, BMP roundtrip matching, 200 rapid
clicks, 1,400 Unicode scalars with zero configured keyboard delay, paced typing,
20 repeated one-pixel crops, and invalid input without pointer movement.
The secondary run repeats the workflows at a negative desktop origin and a
different display density. `get_pixel_color` retains the upstream main-display
contract; use `capture(Some(rect)).color_at(...)` on signed secondary coordinates.

On macOS, text insertion retains a **10 ms minimum interval per Unicode scalar**,
including when the configured delay is zero or the requested CPM exceeds 6,000.
Native text services corrupted unpaced text in the stress test even though the
key events arrived. This interval is an observed mitigation, not a guarantee that
every application accepts arbitrary input rates. Newline and carriage return
characters send native Return events. Other app-level text substitutions and
input methods can still affect the resulting text.

## Native AppKit fixture

```sh
cargo test --test native_macos -- --ignored --nocapture
```

This macOS-only test builds an owned Swift/AppKit window using Xcode command line
tools. It checks native button activation, Unicode and all three multiline typing
APIs, selection/replacement, undo/redo, and left/right Shift held together with
both release orders. No existing user document is opened. The standard Edit menu
uses Cocoa's responder chain, and observations read `NSTextView` contents and
selection. Evidence defaults to `target/release-validation/appkit`, or the
`RUSTY_ROBOT_QA_ARTIFACTS` directory when the runner supplies it.

Four macOS headless tests in `src/platform/macos/modifiers.rs` separately verify
physical left/right modifier state, all 256 held-key combinations, idempotent
presses, and independence between modifier families. They do not require desktop
permissions; the AppKit case verifies their observable native behavior.

## Reference audit

```sh
git clone https://github.com/octalmage/robotjs.git ../robotjs
git -C ../robotjs checkout 0af6073e1f1bcff865ffe52801c71647829cdc1c
node scripts/audit-rust-port.mjs ../robotjs
```

The audit verifies registrations, source hashes, and mapping targets. It does not
execute JavaScript tests or prove behavioral equivalence on untested platforms.

## Diagnostics and cross-target checks

```sh
cargo run --example inspect
cargo run --example inspect -- /tmp/desktop.png
rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-gnu
cargo check --target x86_64-unknown-linux-gnu --all-targets
cargo check --target x86_64-pc-windows-gnu --all-targets
```

The first diagnostic reads permissions, displays and pointer location without
prompting; the second additionally saves a screenshot. Cross-target checking
validates compilation, not actual event injection or capture on those platforms.
