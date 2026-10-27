# Validation record

**Open-source packaging checkpoint, 2026-10-27:** 117 headless tests with PNG /
107 without passed, including four compiled agent-skill examples. Both Clippy
configurations, Rust 1.88 library checks, rustdoc with warnings denied, Cargo
package extraction/build, RustSec audit, 77-case upstream mapping, and archived
benchmark consistency passed locally. Native behavior is recorded separately below.
Remote CI results are available in GitHub Actions.

**Latest headless checkpoint, 2026-10-27:** after search/BMP optimizations,
**113 tests with PNG / 103 without passed**, including two new regressions.
Formatting, macOS Clippy in both feature configurations, and Linux/Windows
cross-target Clippy passed. The 36 opt-in desktop tests were not rerun for these
bitmap-only changes. See [optimization results](optimization-results.md).

**Latest native checkpoint, 2026-10-27:** clean macOS native validation, **37 test
executions passed with no omissions**, plus **111 headless tests with PNG / 101
without**. The left/right modifier regression is fixed. Linux/Windows runtime
validation remains unexecuted; their desktop CI workflow is prepared. See the
[native release checkpoint](#native-release-checkpoint--2026-10-27) below.

## Initial validation — 2026-10-26

Verified on 2026-10-26 on Apple Silicon, macOS 27.0 (26A425), Rust 1.99.1.
The upstream reference remains pinned to
`0af6073e1f1bcff865ffe52801c71647829cdc1c` and unchanged.

| Check | Result |
| --- | --- |
| Cargo tests, PNG enabled | 76 passed, 0 failed; 34 desktop tests opt-in |
| Cargo tests, PNG disabled | 70 passed, 0 failed; 34 desktop tests opt-in |
| Real macOS desktop suite | **34 passed, 0 failed, 0 ignored** in 156.27 seconds |
| Upstream coverage audit | All 77 cases mapped; source hashes unchanged |
| Formatting | Passed |
| Clippy, both feature configurations on macOS | Passed with warnings denied |
| Linux and Windows cross-target checks | Passed with PNG enabled and disabled |
| Linux and Windows cross-target Clippy | Passed with warnings denied |

The Cargo totals include 31 compile-fail API contracts. The desktop suite includes
33 original cases and one new held-modifier regression. Eight additional headless
regressions cover invalid inputs, malformed files and arithmetic boundaries.
Counts from the two feature configurations overlap; they are not additive.

The desktop run used real native input and ScreenCaptureKit capture against the
pinned Target Practice Electron app. A temporary OS virtual monitor at
`(-1280, 0)` exercised the secondary-display case. All assertions ran, including
pixel colors, full-screen origin, scroll directions, shifted symbols, select-all
replacement, Unicode, and held-modifier release. No backend was mocked.

Implementation fixes validated by the suite include macOS Retina pixel sampling,
exact sRGB color preservation, ARM64 scroll argument ordering, modifier state
across calls, and fixture activation across desktop transitions. The test helper
releases its temporary display after the command and shuts down its Electron
processes. Original test assertions were retained.

Linux and Windows GUI behavior has **not** been executed on native hosts. Their
compilation/lint results do not establish runtime parity. CI is configured to run
headless tests on all three OSes when this repository is pushed; that remote CI
has not been run in this session. See [testing.md](testing.md) for reproduction.

## Additional visual verification — 2026-10-27

Added `tests/visual_desktop.rs` and an owned Electron observation window. Eight
workflows exercise actual mouse buttons, smooth motion, held-button drag/drop,
scrolling in both directions on both axes, Unicode and multiline editing,
shortcuts, image-based clicks, input stress, and repeated one-pixel captures.
Each checkpoint saves browser observations, a Rust BMP, and an independent macOS
screenshot under `target/desktop-qa`. Screenshots of dragging, multilingual text,
and the final main-display state were visually inspected.

This pass found and addressed two text-input problems:

- A Unicode LF event was silently dropped by Chromium. All three text APIs now
  inject native Return for LF/CR; multiline text was verified in the actual UI.
- Unpaced text sometimes gained periods or lost spaces despite keydown events
  arriving. macOS Unicode typing now retains a documented 10 ms minimum interval.
  A full main-display workflow passed after this mitigation in **25.84 seconds**,
  including 200 rapid clicks and exact receipt of 1,400 Unicode scalars.

Further assertions for `unicode_tap` and high-CPM multiline text passed on the
negative-origin secondary display, as did dragging, scrolling, editing and
image-based clicking in one run. **The final secondary and full desktop reruns
remain incomplete:** later runs received unrelated pointer movement, clicks and
an `o` keystroke before the typing stage. Another run lost input partway through
typing. Native input was stopped pending an idle desktop. These runs are not
counted as passes, and the earlier 34-test result above is from October 26.
The clean native release checkpoint below now supersedes these interrupted runs.

After the changes, the headless suites still passed (76 with PNG, 70 without),
both macOS Clippy feature configurations passed, Linux and Windows all-target
Clippy passed, and the 77-case reference audit passed. The additional visual test
is opt-in, so headless runs now list **35** ignored desktop tests. No native
Windows/Linux GUI validation was performed.

## Regression expansion — 2026-10-27

Added **31 headless tests**: 13 keyboard fault-injection tests, 8 search and
coordinate tests, and 10 codec tests (6 without PNG). Exhaustive cases cover all
512 binary 3×3 images, every valid rectangular color-search region, and every
binary 2×2 pattern. Codec tests cover 1,904 roundtrips with PNG enabled, malformed
headers, truncation, checksum failures, alpha, and bit-depth conversion.

The new tests first reproduced two input cleanup bugs, then passed with the fixes:
modifier lookup could fail after earlier modifiers were pressed, and cleanup flags
could retain a modifier whose press failed. Chords now resolve modifiers before
emitting events and update active flags only after a successful press. Tests
exercise each failure position and verify that subsequent cleanup is attempted.

| Check after regression changes | Result |
| --- | --- |
| Headless Cargo tests, PNG enabled | **107 passed**, 0 failed; 35 desktop tests ignored |
| Headless Cargo tests, PNG disabled | **97 passed**, 0 failed; 35 desktop tests ignored |
| Formatting and macOS Clippy, both features | Passed |
| Linux/Windows all-target Clippy, both features | Passed (cross-target; not runtime validation) |
| Reference audit | All 77 upstream cases remain mapped |

The existing CI workflow includes these tests through Cargo discovery. Remote CI
was not run. No live input was injected during this regression expansion; the
desktop rerun limitations recorded above still apply.

## Native release checkpoint — 2026-10-27

Executed `python3 scripts/validate-desktop.py --secondary` after the modifier
fix. The runner saved `target/desktop-validation/report.json`, suite logs,
application observations, Rust captures, and independent macOS screenshots.
Its final report has `complete: true`, `passed: true`, and `omissions: []`.

| Native check | Result | Test duration |
| --- | --- | --- |
| Original desktop suite, including secondary capture | 34 passed; 0 failed/ignored | 157.75 s |
| Eight visual workflows on the main Retina display | 1 passed; 0 failed/ignored | 26.30 s |
| Same workflows on the negative-origin 1× virtual display | 1 passed; 0 failed/ignored | 25.88 s |
| Native AppKit button, text editing, undo/redo, Unicode and both Shift release orders | 1 passed; 0 failed/ignored | 4.63 s |

These are **37 executions of 36 distinct desktop tests**: the visual workflow is
deliberately repeated on two displays. Main-display captures were 2× in the
independent OS screenshots; the secondary screenshot was 1×. The virtual monitor
was at `(-1280, 0)`. Final inspection confirmed only the original display remained
and no owned fixture process was still running.

The new AppKit case first reproduced `aa` instead of `Aa` when left Shift was
released while right Shift remained held. macOS now tracks all eight physical
modifier keys separately and derives event flags from those keys. Four headless
regressions cover both release orders in every modifier family, repeated presses,
releasing an unheld opposite key, and all 256 held-key combinations. The native
test also confirms both Shift release orders produce `AaAa` after the fix.

After this change, headless validation passed **111 tests with PNG and 101
without**, including 31 compile-fail contracts. There are 36 opt-in desktop tests
on macOS. Formatting, Clippy in both feature modes, Linux/Windows cross-target
Clippy, and the 77-case upstream audit passed.

The manual `Native desktop validation` GitHub workflow now prepares Linux/X11
under Xvfb/Openbox and Windows native runners and uploads evidence on failure or
success. It has **not been remotely executed**. Its single-display runs explicitly
record omitted secondary-display tests. Other remaining release evidence includes
native Windows/Linux results, additional macOS versions and keyboard layouts,
permission denial/revocation, and physical display hot-plug behavior. Passing this
checkpoint establishes the tested macOS scope, not universal application or
cross-platform compatibility.
