//! Additional boundary tests beyond the original RobotJS coverage.
mod support;
use rusty_robot::{Robot, SearchOptions, image};
use support::*;

#[test]
fn metadata_multiplication_overflow_is_rejected_before_indexing() {
    let mut bitmap = bitmap(&[&["123456"]]);
    bitmap.byte_width = isize::MAX as usize;
    bitmap.height = 3;
    error(bitmap.color_at(0, 0), "outside the supported range");
}
#[test]
fn search_bounds_do_not_wrap_or_escape_the_bitmap() {
    let bitmap = bitmap(&[&["123456", "abcdef"]]);
    for options in [
        SearchOptions {
            x: Some(3),
            ..Default::default()
        },
        SearchOptions {
            x: Some(1),
            width: Some(isize::MAX as usize),
            ..Default::default()
        },
        SearchOptions {
            y: Some(1),
            height: Some(1),
            ..Default::default()
        },
    ] {
        error(bitmap.find_color("123456", options), "outside the bitmap");
        error(bitmap.find_image(&bitmap, options), "outside the bitmap");
    }
    assert_eq!(
        bitmap
            .count_color(
                "123456",
                SearchOptions {
                    width: Some(0),
                    ..Default::default()
                }
            )
            .unwrap(),
        0
    );
}
#[test]
fn negative_bitmap_coordinates_are_rejected() {
    error(bitmap(&[&["123456"]]).color_at(-1, 0), "outside the bitmap");
}
#[test]
fn invalid_click_button_has_no_side_effects() {
    let mut spy = MouseSpy::default();
    error(
        bitmap(&[&["123456"]]).click(&mut spy, point(0, 0), None, Some("party"), false),
        "Invalid mouse",
    );
    assert!(spy.moves.is_empty() && spy.clicks.is_empty());
}
#[test]
fn coordinate_rounding_matches_javascript_for_negative_halves() {
    let mut bitmap = bitmap(&[&["123456"]]);
    bitmap.scale_x = 2.0;
    bitmap.scale_y = 2.0;
    assert_eq!(
        bitmap.to_screen_point(point(-1, -3), None).unwrap(),
        point(0, -1)
    );
    bitmap.scale_x = f64::NAN;
    error(bitmap.to_screen_point(point(1, 1), None), "finite");
}
#[test]
fn malformed_bmp_headers_do_not_panic_or_allocate_from_untrusted_dimensions() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("bad.bmp");
    let valid = bmp_bytes(&[&["123456"]]);
    let mut overlap = valid.clone();
    overlap[10..14].copy_from_slice(&0u32.to_le_bytes());
    let mut huge = valid.clone();
    huge[18..22].copy_from_slice(&i32::MAX.to_le_bytes());
    let mut compressed = valid.clone();
    compressed[30..34].copy_from_slice(&1u32.to_le_bytes());
    for bytes in [
        vec![],
        valid[..53].to_vec(),
        valid[..valid.len() - 1].to_vec(),
        overlap,
        huge,
        compressed,
    ] {
        std::fs::write(&path, bytes).unwrap();
        assert!(image::load(&path).is_err());
    }
}
#[test]
fn loads_top_down_bmp_rows() {
    let mut bytes = bmp_bytes(&[&["ff0000"], &["0000ff"]]);
    bytes[22..26].copy_from_slice(&(-2i32).to_le_bytes());
    // The on-disk row order stays fixed; the negative height reverses its interpretation.
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("top-down.bmp");
    std::fs::write(&path, bytes).unwrap();
    let image = image::load(&path).unwrap();
    assert_eq!(image.color_at(0, 0).unwrap(), "0000ff");
    assert_eq!(image.color_at(0, 1).unwrap(), "ff0000");
}
#[test]
fn input_rejects_bad_options_before_native_side_effects() {
    error(Robot.key_tap("a", &["invalid"]), "Invalid key flag");
    error(Robot.mouse_click(Some("invalid"), false), "Invalid mouse");
    error(Robot.move_mouse(i64::MAX, 0), "supported range");
    for speed in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        error(
            Robot.move_mouse_smooth(0, 0, Some(speed)),
            "positive and finite",
        );
    }
    error(
        Robot.type_string_delayed("not typed", 0),
        "greater than zero",
    );
}
