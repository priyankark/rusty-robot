# Implementation plan

## Boundaries

The public contract remains `Robot`, `Bitmap`, `SearchOptions`, and the existing
result/geometry types. Preserve all upstream assertions and the compile-time
adaptations already recorded in the coverage manifest. Implement Rust behavior;
do not load RobotJS or call its addon.

| Responsibility | Location | Ownership and invariants |
| --- | --- | --- |
| Geometry, errors, public exports | `src/lib.rs`, `src/types.rs`, `src/error.rs` | Keep the tested API stable; report errors instead of panics at public boundaries. |
| Bitmap storage and search | `src/bitmap/` | Validate raw metadata, checked arithmetic, BGR/BGRA stride, row-major matches, overlapping image matches, Euclidean RGB tolerance. |
| Image files | `src/image/` | BMP headers/orientation/padding and optional PNG; preserve RGB under transparency. Decode into owned bitmap buffers. |
| Mouse | `src/mouse.rs` | Validate before effects, button defaults, delays, smooth movement with a bounded time budget, image-click effect injection. |
| Keyboard | `src/keyboard.rs` | Named keys, Unicode, modifier lifecycle, typing cadence, explicit down/up state. |
| Screen and permissions | `src/screen.rs`, `src/permissions.rs` | Signed display origins, capture metadata, explicit permission requests; status queries do not prompt. |
| OS adaptation | `src/platform/` | Native event injection/capture behind a shared Rust interface. Keep FFI and OS resource ownership here. |

The `Robot` unit struct preserves the initial API. Delay settings are process-wide
atomics, matching upstream. Input sequences are serialized to prevent modifier
interleaving. Bitmap operations are independent and can run concurrently.

## Execution order and verification

1. Implement bitmap validation/search and coordinate conversion. Run the bitmap
   suite and the image-click/coordinate contracts. First/all/count variants share
   scanning logic, with early termination for first-match and no allocation for counts.
2. Implement BMP and optional PNG. Use the Rust `png` codec rather than importing
   libpng; implement the small BMP codec directly with checked lengths/offsets.
   Run both feature configurations, including malformed-file regressions.
3. Implement native macOS input, display enumeration, screen capture, permissions,
   with scoped ownership for Core Foundation objects. Port RobotJS semantics while
   checking native failures. Exercise the real Target Practice UI.
4. Add Linux/X11 and Windows backends behind the same interface, with explicit
   errors for unsupported environments. Check compilation on available targets;
   record where native execution still requires another machine.
5. Run the full Cargo matrix, compile-fail contracts, coverage audit, formatting,
   and Clippy. Run desktop cases separately on a controlled fixture. Investigate
   failures in implementation rather than weakening assertions.
6. Update README and port notes with actual results, remaining platform/hardware
   limits, and commands to reproduce. No test is declared passed merely because
   it was ignored or could not run.

## Desktop test environment

The current host is an Apple Silicon Mac running macOS 27. Accessibility and
Screen Recording status queries report granted. One display is active. The
secondary-display contract needs two active displays; preserve that requirement.
The GUI target is the pinned Target Practice Electron app. Rust owns its process,
readiness checks, event stream and test assertions. Desktop tests stay opt-in.

## Implemented structure

The functional split above is now implemented. The native backends are
`platform/macos.rs` (CoreGraphics + ScreenCaptureKit), `platform/linux.rs`
(`x11rb` + XTest/RandR), and `platform/windows.rs` (Win32 SendInput + GDI).

The headless feature matrix, compile-time checks and coverage audit pass. Native
macOS verification includes real Target Practice events and a temporary second
OS display at a negative origin. Linux/Windows runtime validation remains separate
from their successful cross-compilation and lint checks. Current commands and
known limits are maintained in `testing.md` and `rust-port.md`.
