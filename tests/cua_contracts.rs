#![cfg(any(feature = "png", feature = "media"))]
use rusty_robot::Bitmap;
#[cfg(feature = "media")]
use rusty_robot::Point;

fn bitmap() -> Bitmap {
    let mut image = vec![0xEE; 24 * 6];
    for y in 0..6 {
        for x in 0..4 {
            let p = y * 24 + x * 4;
            image[p..p + 4].copy_from_slice(&[30, 60, 180, (x * 25) as u8]);
        }
    }
    Bitmap {
        width: 4,
        height: 6,
        byte_width: 24,
        bits_per_pixel: 32,
        bytes_per_pixel: 4,
        image,
        screen_x: -101,
        screen_y: 17,
        scale_x: 2.0,
        scale_y: 3.0,
    }
}
#[cfg(feature = "png")]
#[test]
fn in_memory_png_preserves_rgb_padding_and_fast_mode() {
    for fast in [false, true] {
        let bytes = rusty_robot::image::encode_png(&bitmap(), fast).unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut pixels).unwrap();
        assert_eq!((info.width, info.height), (4, 6));
        assert!(
            pixels[..info.buffer_size()]
                .as_chunks::<3>()
                .0
                .iter()
                .all(|p| *p == [180, 60, 30])
        );
    }
    let mut invalid = bitmap();
    invalid.image.clear();
    assert!(rusty_robot::image::encode_png(&invalid, true).is_err());
}
#[cfg(feature = "media")]
#[test]
fn encoded_mapping_handles_signed_origin_nonuniform_scale_resize_and_cache() {
    use rusty_robot::media::{Format, Options, process_bitmap, resize_encoded};
    let opts = Options {
        width: Some(2),
        format: Format::Png,
        ..Default::default()
    };
    let source = bitmap();
    let first = process_bitmap(&source, opts, None, true).unwrap();
    assert_eq!((first.width, first.height), (2, 3));
    let mapping = first.mapping.unwrap();
    assert_eq!((mapping.scale_x, mapping.scale_y), (1.0, 1.5));
    assert_eq!(
        first.to_screen_point(Point { x: 1, y: 2 }).unwrap(),
        Point { x: -100, y: 18 }
    );
    assert!(first.to_screen_point(Point { x: 2, y: 0 }).is_err());
    assert!(first.to_screen_point(Point { x: -1, y: 0 }).is_err());
    assert!(
        process_bitmap(&source, opts, Some(&first.fingerprint), false)
            .unwrap()
            .data
            .is_none()
    );
    let mut moved = source.clone();
    moved.screen_x += 1;
    assert!(
        process_bitmap(&moved, opts, Some(&first.fingerprint), false)
            .unwrap()
            .data
            .is_some()
    );
    let file = resize_encoded(first.data.as_ref().unwrap(), 2, None, opts).unwrap();
    assert!(file.mapping.is_none());
    assert!(file.to_screen_point(Point::default()).is_err());
    for scale in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        let mut bad = source.clone();
        bad.scale_x = scale;
        assert!(process_bitmap(&bad, opts, None, true).is_err());
    }
}
