use super::{
    CursorGuard,
    support::point,
    target::{TARGET_COLOR, Target},
};
use rusty_robot::{Robot, Size};
fn setup() -> (Target, CursorGuard) {
    let guard = CursorGuard::new();
    Robot.set_mouse_delay(100).unwrap();
    Robot.set_keyboard_delay(100).unwrap();
    (Target::start(true), guard)
}
#[test]
#[ignore = "requires Target Practice, desktop permissions and native backend"]
fn clicks() {
    let _serial = super::serial();
    let (target, _guard) = setup();
    target.focus("button_1");
    let event = target.event("click");
    assert_eq!(event["id"], "button_1");
    assert_eq!(event["type"], "click");
}
#[test]
#[ignore = "requires Target Practice, desktop permissions and native backend"]
fn scrolls_vertically() {
    let _serial = super::serial();
    let (target, _guard) = setup();
    target.focus("textarea_1");
    Robot.scroll_mouse(0, -10).unwrap();
    let event = target.event("scroll");
    assert_eq!(event["id"], "textarea_1");
    assert!(event["scroll_y"].as_f64().unwrap() > 0.0);
    assert_eq!(event["scroll_x"].as_f64().unwrap(), 0.0);
}
#[test]
#[ignore = "requires Target Practice, desktop permissions and native backend"]
fn scrolls_horizontally() {
    let _serial = super::serial();
    let (target, _guard) = setup();
    target.focus("textarea_1");
    Robot.scroll_mouse(-10, 0).unwrap();
    let event = target.event("scroll");
    assert_eq!(event["id"], "textarea_1");
    assert!(event["scroll_x"].as_f64().unwrap() > 0.0);
    assert_eq!(event["scroll_y"].as_f64().unwrap(), 0.0);
}
#[test]
#[ignore = "requires Target Practice, desktop permissions and native backend"]
fn types() {
    let _serial = super::serial();
    let (target, _guard) = setup();
    target.focus("input_1");
    Robot.type_string("hello world").unwrap();
    target.typed("hello world");
}
#[test]
#[ignore = "requires Target Practice, desktop permissions and native backend"]
fn types_shifted_symbols() {
    let _serial = super::serial();
    let (target, _guard) = setup();
    target.focus("input_1");
    let text = "!@#$%^&*()_+{}|:\"<>?";
    Robot.type_string(text).unwrap();
    target.typed(text);
}
#[test]
#[ignore = "requires Target Practice, desktop permissions and native backend"]
fn replaces_selected_input() {
    let _serial = super::serial();
    let (target, _guard) = setup();
    target.focus("input_1");
    Robot.type_string("initial content").unwrap();
    target.typed("initial content");
    let modifier = if cfg!(target_os = "macos") {
        "command"
    } else {
        "control"
    };
    Robot.key_tap("a", &[modifier]).unwrap();
    Robot
        .type_string_delayed("replacement content", 600)
        .unwrap();
    target.typed("replacement content");
}
#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires Target Practice, macOS permissions and native backend"]
fn types_non_ascii_with_unicode_tap_on_macos() {
    let _serial = super::serial();
    let (target, _guard) = setup();
    target.focus("input_1");
    Robot.type_string("x").unwrap();
    target.typed("x");
    Robot.key_tap("backspace", &[]).unwrap();
    Robot.unicode_tap('嗨').unwrap();
    target.typed("嗨");
}
#[test]
#[ignore = "requires Target Practice, desktop permissions and native backend"]
fn accepts_multi_character_key_names() {
    let _serial = super::serial();
    let (target, _guard) = setup();
    target.focus("input_1");
    for _ in 0..10 {
        for key in [
            "tab",
            "escape",
            "space",
            "enter",
            "backspace",
            "home",
            "end",
        ] {
            Robot.key_tap(key, &[]).unwrap();
        }
    }
}
#[test]
#[ignore = "requires Target Practice, screen permission and native backend"]
fn reads_pixel_color() {
    let _serial = super::serial();
    let target = Target::start(false);
    let p = target.point("color_1");
    assert_eq!(Robot.get_pixel_color(p.x, p.y).unwrap(), TARGET_COLOR);
}
#[test]
#[ignore = "requires Target Practice, screen permission and native backend"]
fn full_capture_uses_pixel_color_origin() {
    let _serial = super::serial();
    let target = Target::start(false);
    let p = target.point("color_1");
    let img = Robot.capture(None).unwrap();
    let top_left = point(
        ((p.x as f64 - 25.0) * img.scale_x).round() as i64,
        ((p.y as f64 - 25.0) * img.scale_y).round() as i64,
    );
    let center = point(
        (p.x as f64 * img.scale_x).round() as i64,
        (p.y as f64 * img.scale_y).round() as i64,
    );
    let dimensions = Size {
        width: (50.0 * img.scale_x).round() as usize,
        height: (50.0 * img.scale_y).round() as usize,
    };
    let found = img.find_color(TARGET_COLOR, Default::default()).unwrap();
    assert_eq!(Robot.get_pixel_color(p.x, p.y).unwrap(), TARGET_COLOR);
    assert_eq!(found, Some(top_left));
    assert_eq!(img.color_at(center.x, center.y).unwrap(), TARGET_COLOR);
    assert_eq!(
        img.to_screen_point(found.unwrap(), Some(dimensions))
            .unwrap(),
        p
    );
}

#[test]
#[ignore = "requires Target Practice, desktop permissions and native backend"]
fn held_modifier_is_applied_and_released() {
    let _serial = super::serial();
    let (target, _guard) = setup();
    target.focus("input_1");
    Robot.key_toggle("shift", "down", &[]).unwrap();
    let typed = Robot.key_tap("a", &[]);
    let released = Robot.key_toggle("shift", "up", &[]);
    typed.unwrap();
    released.unwrap();
    Robot.key_tap("a", &[]).unwrap();
    target.typed("Aa");
}
