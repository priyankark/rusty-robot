# Desktop automation recipes

These APIs are unreleased; use a source checkout. For installation, permissions,
basic input, and image search, start with the [Rust API recipes](../skills/rusty-robot/references/rust-api.md).

## Capture a preview

The `media` feature provides JPEG/PNG bytes, resizing, and coordinate mapping:

```sh
cargo run --release --features media --example capture_preview -- preview.jpg
```

[The example](../examples/capture_preview.rs) captures the main display, saves a
JPEG preview, and converts an image point to desktop coordinates. For a region,
pass `Robot.capture(Some(rect))?` to `media::process_bitmap`. Preserve the returned
`Frame` and use `frame.to_screen_point(point)` when locating a click in the resized
image. Coordinates from an old frame may be stale after the UI changes.

For PNG without resizing, the default `png` feature provides
`image::encode_png(&bitmap, true)`; `true` selects faster compression and larger
files. `Bitmap::save` remains available for file output.

## Select a target and wait for focus

Choose the native window ID from `Robot.windows()`. Use `activate` when you want
to switch focus; use `require_active` to reject input when another window is active.

```rust,no_run
use rusty_robot::{Error, Result, Robot, TargetWindow, observation::wait_until};

fn focus_window(id: u64) -> Result<TargetWindow> {
    let target = Robot.windows()?.into_iter()
        .find(|window| window.id == id)
        .ok_or_else(|| Error("Window not found".into()))?;
    target.activate()?;
    let observed = wait_until(
        Default::default(),
        || target.refresh(),
        |window| window.active,
        || false,
    ).map_err(|failure| Error(format!("Focus failed: {:?}", failure.reason)))?;
    observed.value.require_active()?;
    Ok(observed.value)
}
```

macOS activates the owning application; a multi-window app may select a different
window. X11 needs an EWMH window manager. Focus can change after a check, and
foreground status does not identify the focused text field.

## Paste literal text into an editor

`type_string` sends typing events, so an editor may auto-indent newlines. For
literal insertion, use `paste_text` and wait until the editor has consumed the
clipboard. Supply an observer that reports a **new paste event** and the editor's
resulting text; reading text alone can mistake an old value for completion.

```rust,no_run
use rusty_robot::{Error, Result, Robot, observation::wait_until};
use rusty_robot::paste::{PasteClipboard, PasteOutcome};

fn paste_source(
    clipboard: &mut impl PasteClipboard,
    source: &str,
    observe_paste: impl FnMut() -> Result<(bool, String)>,
) -> Result<PasteOutcome> {
    Robot.paste_text(clipboard, source, || {
        wait_until(
            Default::default(),
            observe_paste,
            |(received, actual)| *received && actual == source,
            || false,
        ).map(|_| ())
         .map_err(|failure| Error(format!("Paste failed: {:?}", failure.reason)))
    })
}
```

On macOS, use `paste::SystemClipboard::new()?`. Windows and X11 applications
provide a `PasteClipboard` adapter; keep the X11 selection owner alive while
serving data. Adapters must preserve supported formats and reject unsupported
ones before replacing the clipboard. Restoration runs on errors and unwinding;
`clipboard_restored == false` means another owner replaced the clipboard. macOS
checks the clipboard revision, but cannot make restoration atomic against other
processes. Serialize clipboard writes within your application.

`wait_until` defaults to a five-second timeout and a 50 ms polling interval.
Supply `WaitOptions` to change these, and replace `|| false` with your cancellation
check. Failures include the last observation, attempt count, and elapsed time.
Callbacks must return promptly. For repeated actions, make your predicate check
for a new result rather than accepting an earlier matching state.

## Release held input on cancellation

```rust,no_run
use rusty_robot::{Point, Result, Robot, session::MouseButton};

fn drag(start: Point, path: Vec<Point>, cancelled: impl FnMut() -> bool) -> Result<()> {
    Robot.input_session()?.drag_path(MouseButton::Left, start, path, cancelled)
}
```

`drag_path` releases its button on errors, cancellation, and panic unwinding.
Use `session.hold(HeldInput::Key("shift".into()))` for a scoped held key; call the
guard's `finish()` to receive release errors. Sessions reject overlapping sessions
in the same process. All your input producers must cooperate; legacy Robot calls
and external input are not locked out. Forced process termination cannot run cleanup.

## Choose scroll units

```rust,no_run
use rusty_robot::{Robot, ScrollUnit};
Robot.scroll(0, -3, ScrollUnit::WheelSteps)?; // Three wheel steps down.
Ok::<(), rusty_robot::Error>(())
```

Positive x scrolls left; positive y scrolls up. Wheel steps work on all supported
platforms, with a limit of 1,000 per axis per call. Query `scroll_capabilities()`
before requesting `ScrollUnit::Pixels`, currently supported only on macOS.
Applications and OS settings determine the distance moved. The legacy
`scroll_mouse` uses macOS pixels, Windows wheel data (120 per detent), or X11
wheel-button pairs.
