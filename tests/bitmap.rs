mod support;
use rusty_robot::SearchOptions;
use support::*;

#[test]
fn reads_padded_rows() {
    let img = padded_bitmap(
        &[
            &["ff0000", "00ff00", "0000ff"],
            &["ffff00", "ff00ff", "00ffff"],
        ],
        16,
        4,
        0xee,
    );
    for (y, row) in [
        ["ff0000", "00ff00", "0000ff"],
        ["ffff00", "ff00ff", "00ffff"],
    ]
    .iter()
    .enumerate()
    {
        for (x, color) in row.iter().enumerate() {
            assert_eq!(img.color_at(x as i64, y as i64).unwrap(), *color);
        }
    }
}
#[test]
fn color_search_rect_origins() {
    let img = padded_bitmap(
        &[
            &["111111", "222222", "333333", "444444"],
            &["555555", "666666", "777777", "ff00ff"],
            &["888888", "ff00ff", "999999", "aaaaaa"],
        ],
        20,
        4,
        0xee,
    );
    assert_eq!(
        img.find_color("ff00ff", Default::default()).unwrap(),
        Some(point(3, 1))
    );
    assert_eq!(
        img.find_color(
            "ff00ff",
            SearchOptions {
                x: Some(1),
                y: Some(2),
                width: Some(1),
                height: Some(1),
                ..Default::default()
            }
        )
        .unwrap(),
        Some(point(1, 2))
    );
    let options = SearchOptions {
        x: Some(1),
        y: Some(1),
        width: Some(3),
        height: Some(2),
        ..Default::default()
    };
    assert_eq!(
        img.find_colors("ff00ff", options).unwrap(),
        vec![point(3, 1), point(1, 2)]
    );
    assert_eq!(img.count_color("ff00ff", options).unwrap(), 2);
}
#[test]
fn image_search_padding_and_rect_origins() {
    let haystack = padded_bitmap(
        &[
            &["101010", "101010", "101010", "101010", "101010", "101010"],
            &["101010", "ff0000", "0000ff", "ff0000", "0000ff", "101010"],
            &["101010", "00ff00", "ffff00", "00ff00", "ffff00", "101010"],
            &["101010", "101010", "101010", "101010", "101010", "101010"],
        ],
        28,
        4,
        0xee,
    );
    let needle = padded_bitmap(&[&["ff0000", "0000ff"], &["00ff00", "ffff00"]], 12, 4, 0xee);
    assert_eq!(
        haystack.find_image(&needle, Default::default()).unwrap(),
        Some(point(1, 1))
    );
    let options = SearchOptions {
        x: Some(1),
        y: Some(1),
        width: Some(4),
        height: Some(2),
        ..Default::default()
    };
    assert_eq!(
        haystack.find_images(&needle, options).unwrap(),
        vec![point(1, 1), point(3, 1)]
    );
    assert_eq!(haystack.count_image(&needle, options).unwrap(), 2);
    assert_eq!(
        haystack
            .find_image(
                &needle,
                SearchOptions {
                    x: Some(0),
                    y: Some(0),
                    width: Some(1),
                    height: Some(4),
                    ..Default::default()
                }
            )
            .unwrap(),
        None
    );
}
#[test]
fn overlapping_images_include_final_row_and_column() {
    let haystack = bitmap(&[
        &["aabbcc", "aabbcc", "aabbcc", "101010"],
        &["101010", "101010", "aabbcc", "aabbcc"],
    ]);
    let needle = bitmap(&[&["aabbcc", "aabbcc"]]);
    assert_eq!(
        haystack.find_images(&needle, Default::default()).unwrap(),
        vec![point(0, 0), point(1, 0), point(2, 1)]
    );
    assert_eq!(
        haystack.count_image(&needle, Default::default()).unwrap(),
        3
    );
    assert_eq!(
        haystack
            .find_image(
                &needle,
                SearchOptions {
                    x: Some(1),
                    y: Some(0),
                    ..Default::default()
                }
            )
            .unwrap(),
        Some(point(1, 0))
    );
}
#[test]
fn tolerance_threshold() {
    let first = bitmap(&[&["101010"]]);
    let second = bitmap(&[&["151010"]]);
    for (tolerance, expected) in [(0.01, None), (0.02, Some(point(0, 0)))] {
        let options = SearchOptions {
            tolerance,
            ..Default::default()
        };
        assert_eq!(first.find_color("151010", options).unwrap(), expected);
        assert_eq!(first.find_image(&second, options).unwrap(), expected);
    }
}
#[test]
fn hex_color_validation() {
    let img = bitmap(&[&["123456"]]);
    assert_eq!(
        img.find_color("#123456", Default::default()).unwrap(),
        Some(point(0, 0))
    );
    for color in ["12345", "1234567", "gggggg"] {
        error(
            img.find_color(color, Default::default()),
            "Color must be a 6-digit hex string",
        );
    }
}
#[test]
fn color_tolerance_is_symmetric_for_every_channel() {
    for second in ["111010", "101110", "101011"] {
        for (a, b) in [("101010", second), (second, "101010")] {
            assert_eq!(
                bitmap(&[&[a]])
                    .find_color(
                        b,
                        SearchOptions {
                            tolerance: 0.01,
                            ..Default::default()
                        }
                    )
                    .unwrap(),
                Some(point(0, 0))
            );
        }
    }
}
#[test]
fn image_tolerance_is_symmetric_for_mixed_deltas() {
    let first = bitmap(&[&["101010", "202020"], &["303030", "404040"]]);
    let second = bitmap(&[&["111010", "201f20"], &["303031", "3f4040"]]);
    let options = SearchOptions {
        tolerance: 0.01,
        ..Default::default()
    };
    assert_eq!(
        first.find_image(&second, options).unwrap(),
        Some(point(0, 0))
    );
    assert_eq!(
        second.find_image(&first, options).unwrap(),
        Some(point(0, 0))
    );
}
#[test]
fn search_rect_rejects_native_range_overflow() {
    let img = bitmap(&[&["123456", "abcdef"], &["fedcba", "654321"]]);
    let needle = bitmap(&[&["123456"]]);
    for field in 0..4 {
        let mut options = SearchOptions::default();
        *match field {
            0 => &mut options.x,
            1 => &mut options.y,
            2 => &mut options.width,
            _ => &mut options.height,
        } = Some(usize::MAX);
        error(
            img.find_color("123456", options),
            "outside the supported range",
        );
        error(
            img.find_image(&needle, options),
            "outside the supported range",
        );
    }
}
#[test]
fn rejects_invalid_tolerances() {
    let img = bitmap(&[&["123456"]]);
    for tolerance in [f64::NAN, f64::INFINITY, f64::MAX, -0.01, 1.01] {
        let options = SearchOptions {
            tolerance,
            ..Default::default()
        };
        error(
            img.find_color("123456", options),
            "tolerance must be between 0.0 and 1.0",
        );
        error(
            img.find_image(&img, options),
            "tolerance must be between 0.0 and 1.0",
        );
    }
}
macro_rules! invalid_metadata {
    ($name:ident, $img:ident, $change:block, $message:literal) => {
        #[test] fn $name() { let mut $img = bitmap(&[&["123456"]]); $change error($img.color_at(0,0),$message); }
    }
}
invalid_metadata!(
    metadata_dimensions_outside_native_range,
    img,
    {
        img.width = usize::MAX;
    },
    "width is outside the supported range"
);
invalid_metadata!(
    metadata_bits_do_not_wrap,
    img,
    {
        img.bits_per_pixel = 288;
    },
    "pixel metadata is outside the supported range"
);
invalid_metadata!(
    metadata_bytes_do_not_wrap,
    img,
    {
        img.bits_per_pixel = 24;
        img.bytes_per_pixel = 259;
    },
    "pixel metadata is outside the supported range"
);
invalid_metadata!(
    metadata_consistent_pixel_format,
    img,
    {
        img.bits_per_pixel = 24;
    },
    "must use 24-bit or 32-bit pixels"
);
invalid_metadata!(
    metadata_stride_covers_row,
    img,
    {
        img.byte_width = 3;
    },
    "byteWidth is smaller than the bitmap row size"
);
invalid_metadata!(
    metadata_buffer_covers_rows,
    img,
    {
        img.image = vec![0; 3];
    },
    "image buffer is smaller than byteWidth * height"
);
#[test]
fn all_search_variants_return_negative_results() {
    let img = bitmap(&[&["101010", "202020"], &["303030", "404040"]]);
    let needle = bitmap(&[&["ffffff"]]);
    assert_eq!(img.find_color("ffffff", Default::default()).unwrap(), None);
    assert!(
        img.find_colors("ffffff", Default::default())
            .unwrap()
            .is_empty()
    );
    assert_eq!(img.count_color("ffffff", Default::default()).unwrap(), 0);
    assert_eq!(img.find_image(&needle, Default::default()).unwrap(), None);
    assert!(
        img.find_images(&needle, Default::default())
            .unwrap()
            .is_empty()
    );
    assert_eq!(img.count_image(&needle, Default::default()).unwrap(), 0);
}
