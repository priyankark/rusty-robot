//! Exhaustive small images provide reproducible cases without a random seed or desktop.
use rusty_robot::{Bitmap, Error, MouseOutput, Point, SearchOptions, Size};

fn pixels(width: usize, height: usize, values: &[u8], bpp: usize, padding: usize) -> Bitmap {
    let stride = width * bpp + padding;
    // Padding deliberately looks like a search target. It must never be searched.
    let mut data = vec![255; stride * height];
    for (index, value) in values.iter().enumerate() {
        let offset = index / width * stride + index % width * bpp;
        data[offset..offset + 3].fill(*value);
        if bpp == 4 {
            data[offset + 3] = (index * 31) as u8;
        }
    }
    Bitmap {
        width,
        height,
        byte_width: stride,
        bytes_per_pixel: bpp as u16,
        bits_per_pixel: bpp as u16 * 8,
        image: data,
        screen_x: 0,
        screen_y: 0,
        scale_x: 1.0,
        scale_y: 1.0,
    }
}
fn options(x: usize, y: usize, w: usize, h: usize) -> SearchOptions {
    SearchOptions {
        x: Some(x),
        y: Some(y),
        width: Some(w),
        height: Some(h),
        tolerance: 0.0,
    }
}

#[test]
fn exact_color_search_distinguishes_channel_order_and_ignores_alpha_and_padding() {
    for bpp in [3, 4] {
        let mut bitmap = pixels(3, 2, &[0; 6], bpp, 9);
        let colors = [
            [0x56, 0x34, 0x12],
            [0x12, 0x34, 0x56],
            [0x56, 0x12, 0x34],
            [0x34, 0x56, 0x12],
            [0x56, 0x34, 0x12],
            [0x12, 0x56, 0x34],
        ];
        for (index, color) in colors.iter().enumerate() {
            let offset = index / 3 * bitmap.byte_width + index % 3 * bpp;
            bitmap.image[offset..offset + 3].copy_from_slice(color);
        }
        for y in 0..2 {
            let offset = y * bitmap.byte_width + 3 * bpp;
            bitmap.image[offset..offset + 9].copy_from_slice(&[0x56, 0x34, 0x12].repeat(3));
        }
        for region in [options(0, 0, 3, 2), options(1, 0, 2, 2)] {
            let expected = if region.x == Some(0) {
                vec![Point { x: 0, y: 0 }, Point { x: 1, y: 1 }]
            } else {
                vec![Point { x: 1, y: 1 }]
            };
            assert_eq!(bitmap.find_colors("123456", region).unwrap(), expected);
            assert_eq!(
                bitmap.find_color("123456", region).unwrap(),
                expected.first().copied()
            );
            assert_eq!(
                bitmap.count_color("123456", region).unwrap(),
                expected.len()
            );
        }
    }
}

#[test]
fn exhaustive_color_search_matches_all_rectangles_of_every_binary_3_by_3_image() {
    for mask in 0u32..512 {
        let values: Vec<u8> = (0..9)
            .map(|i| if mask & (1 << i) == 0 { 0 } else { 255 })
            .collect();
        let bitmap = pixels(3, 3, &values, 3 + mask as usize % 2, mask as usize % 5);
        for x in 0..=3 {
            for y in 0..=3 {
                for w in 0..=3 - x {
                    for h in 0..=3 - y {
                        for (value, hex) in [(0, "000000"), (255, "#FFFFFF")] {
                            let expected: Vec<Point> = values
                                .iter()
                                .enumerate()
                                .filter_map(|(i, &v)| {
                                    let (px, py) = (i % 3, i / 3);
                                    (v == value && px >= x && px < x + w && py >= y && py < y + h)
                                        .then_some(Point {
                                            x: px as i64,
                                            y: py as i64,
                                        })
                                })
                                .collect();
                            let region = options(x, y, w, h);
                            assert_eq!(
                                bitmap.find_colors(hex, region).unwrap(),
                                expected,
                                "mask={mask}, region={region:?}"
                            );
                            assert_eq!(
                                bitmap.find_color(hex, region).unwrap(),
                                expected.first().copied()
                            );
                            assert_eq!(bitmap.count_color(hex, region).unwrap(), expected.len());
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn exhaustive_patterns_include_overlaps_edges_and_region_containment() {
    for mask in 0u32..512 {
        let values: Vec<u8> = (0..9)
            .map(|i| if mask & (1 << i) == 0 { 0 } else { 255 })
            .collect();
        let bitmap = pixels(3, 3, &values, 3 + mask as usize % 2, 5);
        for pattern in 0u32..16 {
            let target: Vec<u8> = (0..4)
                .map(|i| if pattern & (1 << i) == 0 { 0 } else { 255 })
                .collect();
            let needle = pixels(2, 2, &target, 4 - mask as usize % 2, 7);
            for region in [
                options(0, 0, 3, 3),
                options(1, 1, 2, 2),
                options(0, 0, 1, 3),
            ] {
                let expected: Vec<Point> = [(0usize, 0usize), (1, 0), (0, 1), (1, 1)]
                    .into_iter()
                    .filter(|&(x, y)| {
                        x >= region.x.unwrap()
                            && y >= region.y.unwrap()
                            && x + 2 <= region.x.unwrap() + region.width.unwrap()
                            && y + 2 <= region.y.unwrap() + region.height.unwrap()
                    })
                    .filter(|&(x, y)| {
                        [
                            values[y * 3 + x],
                            values[y * 3 + x + 1],
                            values[(y + 1) * 3 + x],
                            values[(y + 1) * 3 + x + 1],
                        ] == target.as_slice()
                    })
                    .map(|(x, y)| Point {
                        x: x as i64,
                        y: y as i64,
                    })
                    .collect();
                assert_eq!(
                    bitmap.find_images(&needle, region).unwrap(),
                    expected,
                    "mask={mask}, pattern={pattern}, {region:?}"
                );
                assert_eq!(
                    bitmap.find_image(&needle, region).unwrap(),
                    expected.first().copied()
                );
                assert_eq!(bitmap.count_image(&needle, region).unwrap(), expected.len());
            }
        }
    }
}

#[test]
fn increasing_tolerance_never_removes_a_match() {
    let values: Vec<u8> = (0..=255).collect();
    let bitmap = pixels(16, 16, &values, 4, 11);
    let needle = pixels(1, 1, &[128], 3, 0);
    let mut previous = Vec::new();
    for step in 0..=100 {
        let options = SearchOptions {
            tolerance: step as f64 / 100.0,
            ..Default::default()
        };
        let current = bitmap.find_colors("808080", options).unwrap();
        assert!(previous.iter().all(|p| current.contains(p)));
        assert_eq!(bitmap.find_images(&needle, options).unwrap(), current);
        previous = current;
    }
    assert_eq!(previous.len(), 256);
}

#[test]
fn tolerance_uses_rgb_distance_and_includes_the_exact_boundary() {
    let mut bitmap = pixels(1, 1, &[0], 4, 0);
    bitmap.image[..3].copy_from_slice(&[0, 4, 3]); // RGB distance exactly five.
    for (distance, expected) in [(4.999, 0), (5.0, 1), (5.001, 1)] {
        assert_eq!(
            bitmap
                .count_color(
                    "000000",
                    SearchOptions {
                        tolerance: distance / 442.0,
                        ..Default::default()
                    }
                )
                .unwrap(),
            expected
        );
    }
}

#[test]
fn invalid_tolerance_is_rejected_by_all_search_variants() {
    let bitmap = pixels(1, 1, &[0], 3, 0);
    for tolerance in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.01, 1.01] {
        let options = SearchOptions {
            tolerance,
            ..Default::default()
        };
        assert!(bitmap.find_color("000000", options).is_err());
        assert!(bitmap.find_colors("000000", options).is_err());
        assert!(bitmap.count_color("000000", options).is_err());
        assert!(bitmap.find_image(&bitmap, options).is_err());
        assert!(bitmap.find_images(&bitmap, options).is_err());
        assert!(bitmap.count_image(&bitmap, options).is_err());
    }
}

#[test]
fn coordinate_conversion_matches_integer_reference_for_signed_origins_and_centers() {
    let mut bitmap = pixels(1, 1, &[0], 4, 0);
    for origin in [-1920, -1, 0, 1000] {
        for scale in [1i64, 2, 4] {
            bitmap.screen_x = origin;
            bitmap.screen_y = -origin;
            bitmap.scale_x = scale as f64;
            bitmap.scale_y = scale as f64;
            for size in 0..6 {
                for coordinate in -50..50 {
                    let offset = (coordinate + size / 2 + scale / 2).div_euclid(scale);
                    assert_eq!(
                        bitmap
                            .to_screen_point(
                                Point {
                                    x: coordinate,
                                    y: coordinate
                                },
                                Some(Size {
                                    width: size as usize,
                                    height: size as usize
                                })
                            )
                            .unwrap(),
                        Point {
                            x: origin + offset,
                            y: -origin + offset
                        }
                    );
                }
            }
        }
    }
}

#[derive(Default)]
struct FailingMouse {
    moves: usize,
    clicks: usize,
    fail_move: bool,
}
impl MouseOutput for FailingMouse {
    fn move_mouse(&mut self, _: i64, _: i64) -> rusty_robot::Result<()> {
        self.moves += 1;
        if self.fail_move {
            Err(Error("move failed".into()))
        } else {
            Ok(())
        }
    }
    fn mouse_click(&mut self, _: Option<&str>, _: bool) -> rusty_robot::Result<()> {
        self.clicks += 1;
        Err(Error("click failed".into()))
    }
}
#[test]
fn failed_movement_never_clicks_and_click_errors_are_propagated() {
    let bitmap = pixels(1, 1, &[0], 4, 0);
    for fail_move in [true, false] {
        let mut output = FailingMouse {
            fail_move,
            ..Default::default()
        };
        let error = bitmap
            .click_image(&mut output, &bitmap, Default::default(), None, false)
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            if fail_move {
                "move failed"
            } else {
                "click failed"
            }
        );
        assert_eq!(output.moves, 1);
        assert_eq!(output.clicks, usize::from(!fail_move));
    }
}
#[test]
fn invalid_coordinate_conversion_cannot_move_or_click() {
    let mut bitmap = pixels(1, 1, &[0], 4, 0);
    for scale in [f64::NAN, f64::INFINITY, f64::MIN_POSITIVE] {
        bitmap.scale_x = scale;
        let mut output = FailingMouse::default();
        assert!(
            bitmap
                .click(&mut output, Point { x: 100, y: 1 }, None, None, false)
                .is_err()
        );
        assert_eq!((output.moves, output.clicks), (0, 0));
    }
}
