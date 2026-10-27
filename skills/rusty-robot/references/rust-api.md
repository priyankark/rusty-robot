# Rust API recipes

Use Rust 1.88+. Until registry publication, add:

```toml
[dependencies]
rusty-robot = { git = "https://github.com/priyankark/rusty-robot", tag = "v0.1.0" }
```

Inspect without moving or typing:

```rust,no_run
use rusty_robot::Robot;
fn main() -> rusty_robot::Result<()> {
    println!("Accessibility: {:?}", Robot.get_accessibility_permission()?);
    println!("Capture: {:?}", Robot.get_screen_capture_permission()?);
    println!("Displays: {:?}", Robot.get_displays()?);
    println!("Pointer: {:?}", Robot.get_mouse_pos()?);
    Ok(())
}
```

A `Some(false)` permission result means the relevant capability needs an OS grant.
The matching request methods can prompt; after a grant the host may need a restart.
Do not repeatedly retry an authorization failure in a tight loop.

Capture once, then inspect/search without more OS captures:

```rust,no_run
use rusty_robot::{Robot, SearchOptions};
fn main() -> rusty_robot::Result<()> {
    let bitmap = Robot.capture(None)?;
    println!("RGB: {}", bitmap.color_at(0, 0)?);
    if let Some(pixel) = bitmap.find_color("ff8800", SearchOptions::default())? {
        println!("Bitmap pixel: {pixel:?}");
        println!("Desktop point: {:?}", bitmap.to_screen_point(pixel, None)?);
    }
    Ok(())
}
```

For input into an already focused, owned test window:

```rust,no_run
use rusty_robot::Robot;
fn main() -> rusty_robot::Result<()> {
    let select = if cfg!(target_os = "macos") { "command" } else { "control" };
    Robot.key_tap("a", &[select])?;
    Robot.type_string("Rusty Robot 🌍\nSecond line")?;
    Robot.key_tap("enter", &[])?;
    Ok(())
}
```

For dragging, release even when movement fails:

```rust,no_run
use rusty_robot::Robot;
fn main() -> rusty_robot::Result<()> {
    Robot.move_mouse(100, 100)?;
    Robot.mouse_toggle("down", Some("left"))?;
    let movement = Robot.drag_mouse(200, 200, Some("left"));
    let release = Robot.mouse_toggle("up", Some("left"));
    movement?;
    release?;
    Ok(())
}
```

Coordinates above are examples, not safe defaults for an arbitrary user's screen.
For held keyboard state, use `key_toggle(key, "down"/"up", modifiers)` and preserve
cleanup on error or cancellation. `scroll_mouse(x, y)` takes backend-native units:
inspect the existing application's wheel scaling when migrating it.

BMP support is always present; PNG is the default optional feature. With PNG
disabled, use `.bmp` output. `Bitmap::validate()` checks caller-created metadata;
keep row padding, BGR channel ordering, and scale in buffers you construct.
