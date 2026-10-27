# RobotJS test mapping and Rust contract

Cargo package: `rusty-robot`. Rust import: `rusty_robot`. This is an independent
Git repository. The upstream reference clone is separate, at `../robotjs`, pinned
to `0af6073e1f1bcff865ffe52801c71647829cdc1c`.

The initial failing API stubs have been replaced by Rust implementations. Bitmap,
search and BMP logic are Rust code; PNG uses the Rust `png` crate. Native backends
use macOS APIs, the Rust X11 protocol implementation, and Win32 bindings. The
RobotJS native addon is neither linked nor invoked.

## Coverage

All **77 expanded upstream cases** are mapped in
[`tests/upstream-coverage.json`](../tests/upstream-coverage.json), including
parameterized metadata and PNG cases. Six become compile-time contracts because
integer fields cannot represent NaN, infinity, or fractional values. The other
71 have runtime tests across both PNG feature configurations. Argument-count/type
validation within those cases adds **31 compile-fail examples** in total.

| Upstream | Rust port |
| --- | --- |
| `test/bitmap.js` | `tests/bitmap.rs`, capture cases in `tests/desktop.rs`, type contracts |
| `test/image.js` | `tests/image.rs` |
| `test/keyboard.js`, `test/mouse.js`, `test/screen.js` | `tests/desktop.rs`, type contracts |
| `test/permissions.js` | `tests/permissions.rs` |
| `test/integration/*.js` | `tests/gui/integration.rs` |
| `test/helpers/images.js` | `tests/support/mod.rs`: BGR/BGRA buffers, BMP and PNG generators |
| `test/helpers/targetpractice.js` | `tests/gui/target.rs`: process lifetime, marker and interaction readiness, event waits |

The audit enumerates original registrations without running their bodies,
verifies original file hashes, and checks each mapping target. It does not prove
behavioral correctness. See [testing instructions](testing.md) for reproducible
commands and platform limitations.

## Rust API decisions

- Success markers `1`/`true` become `Ok(())`; exceptions become `Result::Err`.
  Missing search results become `None`, empty vectors, or zero.
- Optional arguments use `Option` or `SearchOptions::default()`. Modifiers use
  slices. Unicode input uses `char` and platform-native text events.
- Wrong arity, non-string colors, and non-integral dimensions/rectangles are
  compile-time errors. Numeric string coercion requires explicit caller parsing.
- Signed `i64` coordinates represent negative display origins; buffer dimensions
  and search rectangles use `usize`. `usize::MAX` exercises native range checks
  in place of JavaScript's `Number.MAX_VALUE`.
- Pixel metadata uses `u16` so the upstream 288/259 narrowing regressions remain
  runtime checks. Every bitmap operation validates metadata before indexing.
- `MouseOutput` supports recording click effects without desktop access. It does
  not simulate image search or coordinate conversion.
- Native mouse and keyboard sequences serialize through a process-wide mutex;
  delay settings remain process-wide, matching RobotJS. Explicit down/up calls
  remain the caller's responsibility to pair.

## Platform details discovered during implementation

macOS captures use ScreenCaptureKit with explicit sRGB/BGRA output at native
Retina density. The Rust crop samples logical pixel centers without interpolation,
matching RobotJS's logical-size captures and preserving exact UI colors. This
avoids filtering artifacts from tiny native capture regions and a second color
conversion. Capturing a small region currently captures its entire containing
display first; optimizing that path is future work.

The GUI fixture retains its pinned upstream version. Rust now waits for its
marker center and corners to settle, terminates Electron gracefully, and focuses
input smoke tests inside its disposable window. Original assertions are retained.
A separate test-only helper can attach a temporary OS virtual display to exercise
negative origins. It is not a production backend or mock.

Linux requires X11/XTest; native Wayland support is future work. Windows uses
thread-scoped DPI awareness and GDI handles released through RAII. Linux and
Windows have passed cross-target checks and linting; native GUI validation still
requires those operating systems. CI is configured for headless tests on all three.

No implementation or test result here constitutes complete RobotJS API parity
outside the recorded test contract. Some untested platform-specific media and
special keys, broader keyboard layouts, and display reconnect behavior require
additional coverage before a stable release.
