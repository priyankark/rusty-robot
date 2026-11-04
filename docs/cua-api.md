# Rust computer-use helpers (unreleased)

These additions address the [Chrome dogfood findings](cua-dogfood.md) and issues
#29–#35. Existing typing, scrolling, capture, and input methods remain available.
They are source APIs until the next package release.

## Literal text and clipboard ownership

`type_string` sends typing events. Editors can auto-indent Return events and
change source code inside multiline string literals. Use `Robot.paste_text`
when the target should receive one paste operation. Supply a `PasteClipboard`
adapter and a bounded callback that observes the target consuming the text:

```rust,no_run
# #[cfg(target_os = "macos")]
# fn main() -> rusty_robot::Result<()> {
use rusty_robot::{Robot, paste::SystemClipboard};
let mut clipboard = SystemClipboard::new()?;
Robot.paste_text(&mut clipboard, "literal\n    text", || {
    // Wait for your application's acknowledgement or exact editor contents.
    // Return an error on cancellation/timeout; never acknowledge on a fixed sleep.
    # Err(rusty_robot::Error("supply a target observation".into()))
})?;
# Ok(())
# }
# #[cfg(not(target_os = "macos"))]
# fn main() {}
```

The included macOS adapter snapshots readable clipboard items and formats before
changing anything. It refuses unreadable formats and checks `changeCount` before
restoring. NSPasteboard has no atomic compare-and-swap; hosts must serialize their
clipboard writers, and an external process can still race the final check/write.
Windows and X11 hosts supply their own adapter; keep an X11 selection owner alive
while serving/restoring data. The native CI fixture demonstrates an Electron host
adapter that explicitly rejects non-text formats before mutation. Its text-value
ownership check is a fixture policy, not a general revision-aware clipboard backend.

Restoration is attempted on errors and panic unwinding. A later clipboard owner
is respected (`clipboard_restored: false`); restoration failures are returned.
Applications can also transform pasted content, so verify exact contents where
required. A blocked completion callback cannot be preempted by this synchronous API.

## Targets, observation, and held input

`Robot.windows()` returns window ID, PID, observed bounds, title, and foreground
status. `TargetWindow.refresh()` checks ID/PID identity; `activate()` explicitly
requests activation; `require_active()` rejects focus loss. Activation can be
denied by the OS. On macOS activation selects the owner application and foreground
status is the frontmost eligible WindowServer window of that application; it does
not identify the focused accessibility control. X11 requires an EWMH window manager.
OS IDs can be recycled, and focus can change between a check and input delivery.

`observation::wait_until` takes a timeout, polling interval, capture callback,
predicate, and cancellation callback. Failure includes the last observation,
attempt count, elapsed time, and reason. It rejects success after the deadline;
callbacks must themselves be bounded. A stateful predicate can require a transition
away from stale output before accepting completion of a new action.

`Robot.input_session()` grants exclusive ownership among cooperating sessions in
the process; nested/concurrent sessions fail immediately. `hold` returns a scoped
key/button guard and `drag_path` accepts points plus cooperative cancellation.
Explicit `finish` reports release errors, and Drop retries failed release once.
Unwinding releases held input and permits another session. This cannot exclude
legacy Robot calls, other processes, process termination, or OS delivery failures.

## Encoded frames and coordinate mapping

The default `png` feature adds `image::encode_png(&bitmap, fast)` for in-memory
RGB PNG data. The file encoder now writes whole rows. Both preserve visible RGB
while ignoring alpha and storage padding.

Enable the **`media`** Cargo feature for `rusty_robot::media::{capture,
process_bitmap, resize_encoded}`. It exposes the existing bounded Rust JPEG/PNG,
resize, hash, and unchanged-frame pipeline through the core package. The workspace
media crate reexports it for existing Node consumers. The feature works without
the default `png` feature.

`Frame.mapping` preserves the signed capture origin and the pixel/logical scale
after resizing. Use `Frame.to_screen_point` for clicks in the encoded frame. It
rejects out-of-bounds points and images decoded from files without capture metadata.
Changed origin or scale invalidates the frame fingerprint even when RGB is unchanged.
Frame identity is not evidence that the application has stayed unchanged since capture.

## Scroll units and small captures

`Robot.scroll(x, y, ScrollUnit::WheelSteps)` requests whole wheel steps, capped at
1,000 per axis per call. Positive x scrolls left; positive y scrolls up. macOS also
supports `ScrollUnit::Pixels`; query `scroll_capabilities` before choosing units.
Unsupported units fail before native input. OS settings and the application control
the eventual distance. The original `scroll_mouse` retains its backend-specific
units: macOS pixels, Windows wheel data (120 per detent), and X11 wheel-button pairs.

macOS 15.2+ retains the direct region capture already introduced in 0.1.3. The
macOS 14 fallback now pads small source rectangles to avoid transparent edges,
then samples the requested interior. Returned bitmap dimensions and logical
coordinate semantics stay the same. Correctness is tested against real fixture
colors, repeated one-pixel captures, and negative-origin displays; timing claims
require separate release-build measurements.

See [testing](testing.md) for the headless matrix and native CI workflows.
