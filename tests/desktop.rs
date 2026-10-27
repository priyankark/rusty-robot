//! Opt-in tests: run on an isolated desktop with --ignored --test-threads=1.
#[path = "gui/integration.rs"]
mod integration;
mod support;
#[path = "gui/target.rs"]
mod target;
use rusty_robot::{Rect, Robot};
use std::time::{Duration, Instant};
use support::*;

/// Restore the cursor even when an assertion unwinds.
static DESKTOP: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn serial() -> std::sync::MutexGuard<'static, ()> {
    DESKTOP.lock().unwrap_or_else(|e| e.into_inner())
}

struct CursorGuard(rusty_robot::Point);
impl CursorGuard {
    fn new() -> Self {
        Self(Robot.get_mouse_pos().unwrap())
    }
}
impl Drop for CursorGuard {
    fn drop(&mut self) {
        let _ = Robot.move_mouse(self.0.x, self.0.y);
    }
}
// Keep smoke-test input inside the disposable fixture, never the user's active app.
fn mouse() -> (CursorGuard, target::Target) {
    let guard = CursorGuard::new();
    Robot.set_mouse_delay(100).unwrap();
    let target = target::Target::start(true);
    target.focus("button_1");
    (guard, target)
}
fn keyboard() -> (CursorGuard, target::Target) {
    let guard = CursorGuard::new();
    Robot.set_keyboard_delay(10).unwrap();
    let target = target::Target::start(true);
    target.focus("input_1");
    (guard, target)
}

#[test]
#[ignore = "requires a desktop and native backend"]
fn initial_mouse_position() {
    let _serial = serial();
    let _position: rusty_robot::Point = Robot.get_mouse_pos().unwrap();
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn moves_mouse() {
    let _serial = serial();
    let _guard = mouse();
    Robot.move_mouse(0, 0).unwrap();
    Robot.move_mouse(100, 100).unwrap();
    assert_eq!(Robot.get_mouse_pos().unwrap(), point(100, 100));
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn moves_mouse_smoothly() {
    let _serial = serial();
    let _guard = mouse();
    Robot.move_mouse_smooth(0, 0, None).unwrap();
    Robot.move_mouse_smooth(100, 100, None).unwrap();
    assert_eq!(Robot.get_mouse_pos().unwrap(), point(100, 100));
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn smooth_movement_has_no_timer_stalls() {
    let _serial = serial();
    let _guard = mouse();
    Robot.move_mouse(0, 100).unwrap();
    let start = Instant::now();
    Robot.move_mouse_smooth(300, 100, Some(3.0)).unwrap();
    assert!(start.elapsed() < Duration::from_secs(2));
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn clicks_mouse() {
    let _serial = serial();
    let _guard = mouse();
    Robot.mouse_click(None, false).unwrap();
    for button in ["left", "middle", "right"] {
        Robot.mouse_click(Some(button), false).unwrap();
    }
    Robot.mouse_click(Some("left"), true).unwrap();
    for button in ["party", "0"] {
        error(Robot.mouse_click(Some(button), false), "Invalid mouse");
    }
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn drags_mouse() {
    let _serial = serial();
    let _guard = mouse();
    Robot.drag_mouse(5, 5, None).unwrap();
    error(Robot.drag_mouse(2, 2, Some("party")), "Invalid mouse");
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn scrolls_mouse() {
    let _serial = serial();
    let _guard = mouse();
    Robot.mouse_click(None, false).unwrap();
    for (x, y) in [
        (0, 120),
        (0, 2400),
        (0, -600),
        (120, 0),
        (2400, 0),
        (-600, 0),
        (-600, -600),
    ] {
        Robot.scroll_mouse(x, y).unwrap();
    }
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn toggles_mouse() {
    let _serial = serial();
    let _guard = mouse();
    Robot.mouse_toggle("up", Some("right")).unwrap();
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn taps_key() {
    let _serial = serial();
    let (_cursor, _target) = keyboard();
    Robot.key_tap("a", &[]).unwrap();
    Robot.key_tap("a", &["control"]).unwrap();
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn taps_all_keys() {
    let _serial = serial();
    let (_cursor, _target) = keyboard();
    for key in "abcdefghijklmnopqrstuvwxyz1234567890,./;'[]\\".chars() {
        Robot.key_tap(&key.to_string(), &[]).unwrap();
    }
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn taps_all_numpad_keys() {
    let _serial = serial();
    let (_cursor, _target) = keyboard();
    for digit in 0..10 {
        let result = Robot.key_tap(&format!("numpad_{digit}"), &[]);
        if cfg!(target_os = "linux") {
            error(result, "Invalid key code");
        } else {
            result.unwrap();
        }
    }
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn taps_unicode() {
    let _serial = serial();
    let (_cursor, _target) = keyboard();
    for c in "rάöち嗨ఝ".chars() {
        Robot.unicode_tap(c).unwrap();
    }
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn toggles_key() {
    let _serial = serial();
    let (_cursor, _target) = keyboard();
    Robot.key_toggle("a", "down", &[]).unwrap();
    Robot.key_toggle("a", "up", &[]).unwrap();
    for key in ["ά", "嗨"] {
        error(
            Robot.key_toggle(key, "down", &[]),
            "Invalid key code specified.",
        );
    }
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn control_shift_right_arrow() {
    let _serial = serial();
    let (_cursor, _target) = keyboard();
    Robot
        .key_toggle("right", "down", &["shift", "control"])
        .unwrap();
    Robot
        .key_toggle("right", "up", &["shift", "control"])
        .unwrap();
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn types_string() {
    let _serial = serial();
    let (_cursor, _target) = keyboard();
    Robot.type_string("Typed rάöち嗨ఝ 1").unwrap();
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn types_string_with_delay() {
    let _serial = serial();
    let (_cursor, _target) = keyboard();
    Robot
        .type_string_delayed("Typed rάöち嗨ఝ with delay 1", 600)
        .unwrap();
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn pixel_color() {
    let _serial = serial();
    hex(&Robot.get_pixel_color(5, 5).unwrap());
    error(
        Robot.get_pixel_color(9_999_999_999_999, 9_999_999_999_999),
        "outside the main screen",
    );
    error(Robot.get_pixel_color(-1, -1), "must be non-negative");
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn screen_size() {
    let _serial = serial();
    let size = Robot.get_screen_size().unwrap();
    assert!(size.width > 0 && size.height > 0);
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn enumerates_displays() {
    let _serial = serial();
    let displays = Robot.get_displays().unwrap();
    assert!(!displays.is_empty());
    let display = &displays[0];
    let _: (u64, i64, i64, usize, usize, bool) = (
        display.id,
        display.x,
        display.y,
        display.width,
        display.height,
        display.is_main,
    );
}
#[test]
#[ignore = "requires two active displays and native backend"]
fn captures_secondary_display_at_signed_origin() {
    let _serial = serial();
    let displays = Robot.get_displays().unwrap();
    let display = displays
        .iter()
        .find(|d| !d.is_main && (d.x < 0 || d.y < 0))
        .or_else(|| displays.iter().find(|d| !d.is_main))
        .expect("requires at least two active displays");
    let width = display.width.min(8);
    let height = display.height.min(8);
    let img = Robot
        .capture(Some(Rect {
            x: display.x,
            y: display.y,
            width,
            height,
        }))
        .unwrap();
    assert_eq!((img.screen_x, img.screen_y), (display.x, display.y));
    assert!(img.scale_x > 0.0 && img.scale_y > 0.0);
    assert!((img.scale_x - img.width as f64 / width as f64).abs() < 0.5e-10);
    assert!((img.scale_y - img.height as f64 / height as f64).abs() < 0.5e-10);
    hex(&img.color_at(0, 0).unwrap());
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn bitmap_parameters() {
    let _serial = serial();
    let img = Robot.capture(None).unwrap();
    let _: (usize, usize, usize, u16, u16, Vec<u8>) = (
        img.width,
        img.height,
        img.byte_width,
        img.bits_per_pixel,
        img.bytes_per_pixel,
        img.image,
    );
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn bitmap_specific_size() {
    let _serial = serial();
    let img = Robot
        .capture(Some(Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        }))
        .unwrap();
    let scaled_size = 10.0 * (img.width as f64 / 10.0);
    assert_eq!(img.height as f64, scaled_size);
    assert_eq!(img.width as f64, scaled_size);
}
#[test]
#[ignore = "requires a desktop and native backend"]
fn captured_bitmap_color_and_bounds() {
    let _serial = serial();
    let img = Robot.capture(None).unwrap();
    hex(&img.color_at(0, 0).unwrap());
    let size = Robot.get_screen_size().unwrap();
    let scale = img.width as f64 / size.width as f64;
    let width = (size.width as f64 * scale) as i64;
    let height = (size.height as f64 * scale) as i64;
    error(img.color_at(0, height), "are outside the bitmap");
    img.color_at(0, height - 1).unwrap();
    error(img.color_at(width, 0), "are outside the bitmap");
    error(img.color_at(9_999_999_999_999, 0), "are outside the bitmap");
    error(img.color_at(0, 9_999_999_999_999), "are outside the bitmap");
}
