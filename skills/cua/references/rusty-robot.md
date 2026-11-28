# Rusty Robot backend

Reuse an existing Rust consumer when available. For a new one, use Rust 1.88+
and add this dependency to a Cargo binary project:

```toml
[dependencies]
rusty-robot-desktop = { git = "https://github.com/priyankark/rusty-robot", rev = "0722577c693b99934d627f49cc62e9e7d3fa99dc", features = ["media"] }
```

This commit includes the automation helpers added after the published 0.1.3 API.
Pin the revision actually used; do not assume 0.1.3 has these helpers. No Node.js,
Playwright, CDP, or model-provider SDK is needed. The host only needs to execute
Rust and show the model the captured PNG/JPEG. Do not silently substitute a
different input backend or injected browser events for native desktop actions.

## Check the desktop

```rust,no_run
use rusty_robot::Robot;

fn main() -> rusty_robot::Result<()> {
    println!("Accessibility: {:?}", Robot.get_accessibility_permission()?);
    println!("Capture: {:?}", Robot.get_screen_capture_permission()?);
    println!("Displays: {:?}", Robot.get_displays()?);
    println!("Pointer: {:?}", Robot.get_mouse_pos()?);
    for window in Robot.windows()? {
        println!("{window:?}");
    }
    Ok(())
}
```

| Platform | Requirements |
| --- | --- |
| macOS 14+ | Accessibility for input; Screen Recording for capture |
| Windows | An unlocked interactive desktop; matching privileges for elevated apps |
| Linux | X11/XTest and `DISPLAY`; EWMH window manager for target APIs; native Wayland unsupported |

`Some(false)` is a denied permission. `None` means there is no corresponding
permission-query API, not that permission is granted. Permission-request methods
can prompt; follow the host's grant flow rather than repeatedly retrying input.

## Capture and map coordinates

```rust,no_run
use rusty_robot::{Point, Robot, image};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bitmap = Robot.capture(None)?;
    std::fs::write("observation.png", image::encode_png(&bitmap, true)?)?;
    println!("{}x{} origin=({}, {}) scale=({}, {})",
        bitmap.width, bitmap.height, bitmap.screen_x, bitmap.screen_y,
        bitmap.scale_x, bitmap.scale_y);
    // Example location only: choose the real point after inspecting this image.
    let image_point = Point { x: 100, y: 80 };
    let desktop_point = bitmap.to_screen_point(image_point, None)?;
    println!("Mapped point: {desktop_point:?}");
    Ok(())
}
```

Keep the bitmap and metadata available across the host's observation/action turns;
do not map a point against a newly captured, different image. Validate that the
chosen point lies inside the image before converting it with `Bitmap`.

- `capture(None)` selects the main display, not all monitors. Use `capture(Some(rect))`
  for a signed logical region; it must fit inside one display.
- `Bitmap` coordinates are image pixels. Desktop positions are signed logical
  coordinates. Preserve negative origins and separate x/y scales.
- For resized JPEG/PNG, call `media::process_bitmap(&bitmap, options, None, true)`
  and retain the returned `Frame`. Use **that frame's** `to_screen_point`, since
  the original bitmap's mapping does not include the resize.
- `Frame.data == None` means the previous fingerprint matched; it is not a blank
  image. Request a forced refresh when the host lacks the prior image. Images
  decoded from files lack desktop mapping unless the consumer retained it separately.
- Raw storage is BGR/BGRA with row padding. Use the encoder rather than treating
  `bitmap.image` as packed RGB. PNG support is on by default; `media` adds JPEG/resize.

## Choose the action API

| Task | Rust API and contract |
| --- | --- |
| Identify/check a window | `Robot.windows()`, then `TargetWindow::refresh()` and `require_active()` |
| Switch focus | `target.activate()`, then poll `target.refresh()?.active`; macOS activates the owner app |
| Click | `Robot.move_mouse(x, y)?`, then `Robot.mouse_click(Some("left"), false)?` |
| Shortcut | `Robot.key_tap("l", &["command"])?` on macOS; use `"control"` on Windows/Linux |
| Type | `Robot.type_string(text)?`; Return events may trigger editor auto-indent |
| Literal paste | `Robot.paste_text(&mut clipboard, text, consumed)?`; `consumed` must acknowledge this paste before restoration |
| Drag | `Robot.input_session()?.drag_path(MouseButton::Left, start, path, cancelled)?` |
| Hold a key | `session.hold(HeldInput::Key("shift".into()))?`; call `finish()` to receive release errors |
| Scroll | `Robot.scroll(0, -3, ScrollUnit::WheelSteps)?` scrolls down; pixels require capability support |
| Wait for an observation | `observation::wait_until(options, capture, accept, cancelled)` |

`MouseButton` and `HeldInput` live in `rusty_robot::session`. Key names include
`enter` (not `return`), `command`, `control`, `alt`, and `shift`. Low-level
`drag_mouse` does not press/release a button. `type_string`'s macOS pacing floor
still applies with zero configured delay; event delivery is not exact-text proof.

`paste::SystemClipboard` is available on macOS. Windows/X11 need a host
`PasteClipboard` adapter. Preserve supported formats, reject unsupported formats
before mutation, and keep the X11 selection owner alive. Never restore over a
detected replacement owner. macOS revision checks cannot make restoration atomic
against other processes. If the host cannot acknowledge consumption of this paste,
do not replace that acknowledgment with an arbitrary sleep.

Wheel steps work on all supported platforms; `ScrollUnit::Pixels` is macOS-only.
Positive x scrolls left and positive y scrolls up. Legacy `scroll_mouse` uses
backend-native units, so do not use its integers as cross-platform pixel deltas.

Polling returns the last observation, attempt count, elapsed time, and failure
reason. Default timeout/interval are five seconds/50 ms. Callbacks must be bounded;
the synchronous helper cannot preempt a blocked callback or invoke the model itself.

For complete helper recipes, see the
[pinned automation guide](https://github.com/priyankark/rusty-robot/blob/0722577c693b99934d627f49cc62e9e7d3fa99dc/docs/automation.md).
