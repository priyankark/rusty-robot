mod support;
use rusty_robot::{SearchOptions, Size, image};
use std::{fs, path::Path};
use support::*;

#[test]
fn loads_bmp_as_search_bitmap() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("needle.bmp");
    fs::write(
        &path,
        bmp_bytes(&[&["ff0000", "0000ff"], &["00ff00", "ffff00"]]),
    )
    .unwrap();
    let haystack = padded_bitmap(
        &[
            &["111111", "111111", "111111", "111111"],
            &["111111", "ff0000", "0000ff", "111111"],
            &["111111", "00ff00", "ffff00", "111111"],
        ],
        20,
        4,
        0,
    );
    let needle = image::load(&path).unwrap();
    assert_eq!((needle.width, needle.height), (2, 2));
    assert_eq!(needle.color_at(0, 0).unwrap(), "ff0000");
    assert_eq!(needle.color_at(1, 1).unwrap(), "ffff00");
    assert_eq!(
        haystack.find_image(&needle, Default::default()).unwrap(),
        Some(point(1, 1))
    );
}
#[test]
fn rejects_four_character_load_extensions() {
    error(
        image::load(Path::new("not-an-image.jpeg")),
        "Unsupported image type",
    );
}
#[test]
fn saves_bmp_with_consistent_headers_and_pixels() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("saved.bmp");
    let rows: &[&[&str]] = &[
        &["112233", "445566", "778899"],
        &["aabbcc", "ddeeff", "123456"],
    ];
    image::save(&padded_bitmap(rows, 12, 3, 0), &path).unwrap();
    assert!(path.exists());
    let bytes = fs::read(&path).unwrap();
    let u32_at = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
    let declared_size = u32_at(2);
    let pixel_offset = u32_at(10);
    let dib_size = u32_at(14);
    let width = i32::from_le_bytes(bytes[18..22].try_into().unwrap());
    let height = i32::from_le_bytes(bytes[22..26].try_into().unwrap()).unsigned_abs() as usize;
    let bpp = u16::from_le_bytes(bytes[28..30].try_into().unwrap()) as usize / 8;
    assert!(width > 0);
    let width = width as usize;
    let stride = (width * bpp + 3) & !3;
    let image_size = u32_at(34);
    assert_eq!(bytes.len(), declared_size);
    assert_eq!(pixel_offset, 14 + dib_size);
    assert_eq!(pixel_offset, 54);
    assert_eq!(stride, 12);
    assert!(stride > width * bpp);
    assert_eq!(image_size, stride * height);
    assert_eq!(declared_size, pixel_offset + image_size);
    let reloaded = image::load(&path).unwrap();
    for (y, row) in rows.iter().enumerate() {
        for (x, color) in row.iter().enumerate() {
            assert_eq!(reloaded.color_at(x as i64, y as i64).unwrap(), *color);
        }
    }
}
#[cfg(feature = "png")]
#[test]
fn saves_and_reloads_png() {
    const { assert!(image::SUPPORTS_PNG) };
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("saved.png");
    padded_bitmap(&[&["112233", "445566"], &["778899", "aabbcc"]], 12, 3, 0)
        .save(&path)
        .unwrap();
    assert!(path.exists());
    let reloaded = image::load(&path).unwrap();
    assert_eq!(reloaded.color_at(0, 1).unwrap(), "778899");
    assert_eq!(reloaded.color_at(1, 0).unwrap(), "445566");
}
#[cfg(feature = "png")]
fn decode_png(
    kind: u8,
    rows: &[&[u8]],
    palette: Option<&[u8]>,
    transparency: Option<&[u8]>,
    expected: &[&[&str]],
) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("format.png");
    fs::write(&path, png_bytes(kind, rows, palette, transparency)).unwrap();
    let img = image::load(&path).unwrap();
    for (y, row) in expected.iter().enumerate() {
        for (x, color) in row.iter().enumerate() {
            assert_eq!(img.color_at(x as i64, y as i64).unwrap(), *color);
        }
    }
}
#[cfg(feature = "png")]
const COLORS: &[&[&str]] = &[&["ff0000", "00ff00"], &["0000ff", "ffff00"]];
#[cfg(feature = "png")]
const GRAYS: &[&[&str]] = &[&["101010", "808080"], &["e0e0e0", "404040"]];
#[cfg(feature = "png")]
const PALETTE: &[u8] = &[255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0];
#[cfg(feature = "png")]
#[test]
fn decodes_rgb_png() {
    decode_png(
        2,
        &[&[255, 0, 0, 0, 255, 0], &[0, 0, 255, 255, 255, 0]],
        None,
        None,
        COLORS,
    );
}
#[cfg(feature = "png")]
#[test]
fn decodes_rgba_png() {
    decode_png(
        6,
        &[
            &[255, 0, 0, 0, 0, 255, 0, 0x55],
            &[0, 0, 255, 0xaa, 255, 255, 0, 255],
        ],
        None,
        None,
        COLORS,
    );
}
#[cfg(feature = "png")]
#[test]
fn decodes_grayscale_png() {
    decode_png(0, &[&[0x10, 0x80], &[0xe0, 0x40]], None, None, GRAYS);
}
#[cfg(feature = "png")]
#[test]
fn decodes_grayscale_alpha_png() {
    decode_png(
        4,
        &[&[0x10, 0, 0x80, 0x55], &[0xe0, 0xaa, 0x40, 255]],
        None,
        None,
        GRAYS,
    );
}
#[cfg(feature = "png")]
#[test]
fn decodes_palette_png() {
    decode_png(3, &[&[0, 1], &[2, 3]], Some(PALETTE), None, COLORS);
}
#[cfg(feature = "png")]
#[test]
fn decodes_transparent_palette_png() {
    decode_png(
        3,
        &[&[0, 1], &[2, 3]],
        Some(PALETTE),
        Some(&[0, 0x55, 0xaa, 255]),
        COLORS,
    );
}
#[cfg(not(feature = "png"))]
#[test]
fn rejects_png_when_disabled() {
    const { assert!(!image::SUPPORTS_PNG) };
    error(
        image::load(Path::new("example.png")),
        "PNG support is not enabled",
    );
    error(
        bitmap(&[&["123456"]]).save(Path::new("example.png")),
        "PNG support is not enabled",
    );
}
#[test]
fn rejects_four_character_save_extensions() {
    error(
        image::save(&bitmap(&[&["123456"]]), Path::new("not-an-image.jpeg")),
        "Unsupported image type",
    );
}
#[test]
fn capture_to_screen_coordinates() {
    let mut img = bitmap(&[&["123456", "abcdef"]]);
    img.screen_x = 100;
    img.screen_y = 200;
    img.scale_x = 2.0;
    img.scale_y = 2.0;
    assert_eq!(
        img.to_screen_point(point(20, 10), None).unwrap(),
        point(110, 205)
    );
    assert_eq!(
        img.to_screen_point(
            point(20, 10),
            Some(Size {
                width: 2,
                height: 2
            })
        )
        .unwrap(),
        point(111, 206)
    );
}
#[test]
fn target_centers_with_negative_origins_and_nonuniform_scaling() {
    let mut img = bitmap(&[&["123456"]]);
    img.screen_x = -1440;
    img.screen_y = -900;
    img.scale_x = 1.5;
    img.scale_y = 2.0;
    assert_eq!(
        img.to_screen_point(
            point(300, 200),
            Some(Size {
                width: 48,
                height: 40
            })
        )
        .unwrap(),
        point(-1224, -790)
    );
}
#[test]
fn find_colors_crosses_rows() {
    let img = bitmap(&[
        &["ff0000", "111111", "111111"],
        &["111111", "111111", "ff0000"],
        &["111111", "ff0000", "111111"],
    ]);
    assert_eq!(
        img.find_colors("ff0000", Default::default()).unwrap(),
        vec![point(0, 0), point(2, 1), point(1, 2)]
    );
}
#[test]
fn find_images_crosses_rows() {
    let img = bitmap(&[
        &["aabbcc", "111111", "111111"],
        &["111111", "111111", "aabbcc"],
        &["111111", "aabbcc", "111111"],
    ]);
    assert_eq!(
        img.find_images(&bitmap(&[&["aabbcc"]]), Default::default())
            .unwrap(),
        vec![point(0, 0), point(2, 1), point(1, 2)]
    );
}
#[test]
fn missing_image_does_not_move_or_click() {
    let mut spy = MouseSpy::default();
    assert_eq!(
        bitmap(&[&["123456"]])
            .click_image(
                &mut spy,
                &bitmap(&[&["abcdef"]]),
                Default::default(),
                None,
                false
            )
            .unwrap(),
        None
    );
    assert!(spy.moves.is_empty());
    assert!(spy.clicks.is_empty());
}
#[test]
fn direct_double_click_defaults_to_left() {
    let mut spy = MouseSpy::default();
    bitmap(&[&["123456"]])
        .click(&mut spy, point(0, 0), None, None, true)
        .unwrap();
    assert_eq!(spy.clicks, vec![(Some("left".into()), true)]);
}
#[test]
fn image_double_click_defaults_to_left() {
    let mut img = padded_bitmap(
        &[
            &["111111", "111111", "111111", "111111"],
            &["111111", "ff0000", "0000ff", "111111"],
            &["111111", "00ff00", "ffff00", "111111"],
        ],
        20,
        4,
        0,
    );
    let needle = padded_bitmap(&[&["ff0000", "0000ff"], &["00ff00", "ffff00"]], 12, 4, 0);
    img.screen_x = 10;
    img.screen_y = 20;
    img.scale_x = 2.0;
    img.scale_y = 2.0;
    let mut spy = MouseSpy::default();
    assert_eq!(
        img.click_image(
            &mut spy,
            &needle,
            SearchOptions {
                tolerance: 0.0,
                ..Default::default()
            },
            None,
            true
        )
        .unwrap(),
        Some(point(1, 1))
    );
    assert_eq!(spy.moves, vec![point(11, 21)]);
    assert_eq!(spy.clicks, vec![(Some("left".into()), true)]);
}
