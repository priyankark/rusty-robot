mod support;
use rusty_robot::{Bitmap, image};
use std::fs;
use support::*;

fn generated(width: usize, height: usize, bpp: usize, padding: usize) -> Bitmap {
    let stride = width * bpp + padding;
    let mut image = vec![0xa5; stride * height];
    for y in 0..height {
        for x in 0..width {
            let i = y * stride + x * bpp;
            image[i..i + 3].copy_from_slice(&[
                (x * 17 + y * 19) as u8,
                (x * 31 + y * 7) as u8,
                (x * 13 + y * 43) as u8,
            ]);
            if bpp == 4 {
                image[i + 3] = (x * 41 + y * 29) as u8;
            }
        }
    }
    Bitmap {
        width,
        height,
        byte_width: stride,
        bytes_per_pixel: bpp as u16,
        bits_per_pixel: bpp as u16 * 8,
        image,
        screen_x: -1920,
        screen_y: 40,
        scale_x: 2.0,
        scale_y: 1.5,
    }
}
fn check_roundtrips(extension: &str) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join(format!("roundtrip.{extension}"));
    for width in 1..=17 {
        for height in 1..=7 {
            for bpp in [3, 4] {
                for padding in 0..=3 {
                    let source = generated(width, height, bpp, padding);
                    source.save(&path).unwrap();
                    let loaded = image::load(&path).unwrap();
                    assert_eq!((loaded.width, loaded.height), (width, height));
                    for y in 0..height {
                        for x in 0..width {
                            let i = y * source.byte_width + x * bpp;
                            let j = y * loaded.byte_width + x * 4;
                            assert_eq!(
                                &loaded.image[j..j + 3],
                                &source.image[i..i + 3],
                                "{extension}: {width}x{height}, bpp={bpp}, padding={padding}, ({x},{y})"
                            );
                        }
                    }
                    // Image files have no screen-coordinate provenance.
                    assert_eq!((loaded.screen_x, loaded.screen_y), (0, 0));
                    assert_eq!((loaded.scale_x, loaded.scale_y), (1.0, 1.0));
                }
            }
        }
    }
}
#[test]
fn bmp_roundtrips_all_padding_alignments_and_bgr_bgra_sources() {
    check_roundtrips("bmp");
}

#[test]
fn bmp_saved_bytes_preserve_orientation_channels_and_zero_padding() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("encoded.bmp");
    for width in 1..=4 {
        let top = ["123456", "abcdef", "ff0000", "00ff00"];
        let bottom = ["654321", "fedcba", "0000ff", "ffffff"];
        let rows: &[&[&str]] = &[&top[..width], &bottom[..width]];
        let expected = bmp_bytes(rows);
        for bpp in [3, 4] {
            let mut source = padded_bitmap(rows, width * usize::from(bpp) + 7, bpp, 0xa5);
            if bpp == 4 {
                for y in 0..2 {
                    for x in 0..width {
                        source.image[y * source.byte_width + x * 4 + 3] = (x * 41 + y * 73) as u8;
                    }
                }
            }
            source.save(&path).unwrap();
            assert_eq!(
                fs::read(&path).unwrap(),
                expected,
                "width={width}, bpp={bpp}"
            );
        }
    }
}
#[cfg(feature = "png")]
#[test]
fn png_roundtrips_all_padding_alignments_and_bgr_bgra_sources() {
    check_roundtrips("png");
}

#[test]
fn every_truncation_before_the_end_of_bmp_pixels_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("truncated.bmp");
    let bytes = bmp_bytes(&[
        &["123456", "abcdef", "000000"],
        &["ff0000", "00ff00", "0000ff"],
    ]);
    for length in 0..bytes.len() {
        fs::write(&path, &bytes[..length]).unwrap();
        assert!(
            image::load(&path).is_err(),
            "accepted {length}/{} bytes",
            bytes.len()
        );
    }
}

#[test]
fn bmp_rejects_invalid_dimensions_planes_formats_and_overlapping_dib_headers() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("header.bmp");
    let valid = bmp_bytes(&[&["123456", "abcdef"]]);
    let patches = [
        (18, 0u32.to_le_bytes().to_vec()),
        (18, (-1i32).to_le_bytes().to_vec()),
        (22, 0u32.to_le_bytes().to_vec()),
        (22, i32::MIN.to_le_bytes().to_vec()),
        (26, 0u16.to_le_bytes().to_vec()),
        (26, 2u16.to_le_bytes().to_vec()),
        (28, 16u16.to_le_bytes().to_vec()),
        (28, 8u16.to_le_bytes().to_vec()),
        (14, 39u32.to_le_bytes().to_vec()),
        (14, u32::MAX.to_le_bytes().to_vec()),
        (10, u32::MAX.to_le_bytes().to_vec()),
        (30, 3u32.to_le_bytes().to_vec()),
    ];
    for (offset, patch) in patches {
        let mut bytes = valid.clone();
        bytes[offset..offset + patch.len()].copy_from_slice(&patch);
        fs::write(&path, bytes).unwrap();
        assert!(image::load(&path).is_err(), "header offset {offset}");
    }
}

#[test]
fn bmp_32_bit_top_down_and_bottom_up_preserve_alpha_and_channel_order() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("rgba.bmp");
    // Independent, hand-encoded rows. Includes hidden RGB under zero alpha.
    let top = [0x33, 0x22, 0x11, 0, 0x66, 0x55, 0x44, 127];
    let bottom = [0x99, 0x88, 0x77, 128, 0xcc, 0xbb, 0xaa, 255];
    for height in [2i32, -2] {
        let mut bytes = bmp_bytes(&[&["000000", "000000"], &["000000", "000000"]]);
        bytes[22..26].copy_from_slice(&height.to_le_bytes());
        bytes[28..30].copy_from_slice(&32u16.to_le_bytes());
        bytes[54..].copy_from_slice(&if height < 0 {
            [top, bottom].concat()
        } else {
            [bottom, top].concat()
        });
        fs::write(&path, bytes).unwrap();
        assert_eq!(image::load(&path).unwrap().image, [top, bottom].concat());
    }
}

#[test]
fn malformed_bitmap_save_does_not_truncate_an_existing_file() {
    let temp = tempfile::tempdir().unwrap();
    for extension in ["bmp", "png"] {
        let path = temp.path().join(format!("existing.{extension}"));
        fs::write(&path, b"keep this content").unwrap();
        let mut source = generated(3, 2, 4, 2);
        source.image.truncate(1);
        assert!(source.save(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"keep this content");
    }
}

#[test]
fn bmp_header_bit_mutations_either_decode_consistently_or_return_an_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mutated.bmp");
    let valid = bmp_bytes(&[&["112233", "445566"], &["778899", "aabbcc"]]);
    for offset in 0..54 {
        for bit in 0..8 {
            let mut bytes = valid.clone();
            bytes[offset] ^= 1 << bit;
            fs::write(&path, bytes).unwrap();
            if let Ok(decoded) = image::load(&path) {
                decoded.validate().unwrap();
                assert!(
                    decoded.width <= 5 && decoded.height <= 5,
                    "unbounded dimensions from small file"
                );
                decoded
                    .color_at(decoded.width as i64 - 1, decoded.height as i64 - 1)
                    .unwrap();
            }
        }
    }
}

#[cfg(feature = "png")]
#[test]
fn png_rejects_corrupt_header_and_compressed_data_crc() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("crc.png");
    let valid = png_bytes(
        2,
        &[&[1, 2, 3, 4, 5, 6], &[7, 8, 9, 10, 11, 12]],
        None,
        None,
    );
    let idat = valid.windows(4).position(|v| v == b"IDAT").unwrap();
    let length = u32::from_be_bytes(valid[idat - 4..idat].try_into().unwrap()) as usize;
    for offset in [29, idat + 4 + length] {
        let mut bytes = valid.clone();
        bytes[offset] ^= 1;
        fs::write(&path, bytes).unwrap();
        assert!(image::load(&path).is_err(), "CRC at {offset}");
    }
    // next_frame may not require IEND, but the compressed frame must be complete.
    for length in 0..idat + 4 + length {
        fs::write(&path, &valid[..length]).unwrap();
        assert!(image::load(&path).is_err(), "truncated PNG at {length}");
    }
}

#[cfg(feature = "png")]
#[test]
fn png_sixteen_bit_channels_normalize_to_the_high_byte() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("sixteen.png");
    let mut encoded = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut encoded, 2, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Sixteen);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0, 0, 0xff, 0xff, 0, 0, 0x80, 0xff, 0xff, 0xff,
            ])
            .unwrap();
    }
    fs::write(&path, encoded).unwrap();
    let decoded = image::load(&path).unwrap();
    assert_eq!(decoded.image, [0x9a, 0x56, 0x12, 0, 0x80, 0, 0xff, 0xff]);
}

#[cfg(feature = "png")]
#[test]
fn png_one_bit_palette_expands_transparency_without_losing_hidden_rgb() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("one-bit.png");
    let mut encoded = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut encoded, 3, 1);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::One);
        encoder.set_palette(vec![0x12, 0x34, 0x56, 0xab, 0xcd, 0xef]);
        encoder.set_trns(vec![0, 255]);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[0b0100_0000]).unwrap();
    }
    fs::write(&path, encoded).unwrap();
    let decoded = image::load(&path).unwrap();
    assert_eq!(
        decoded.image,
        [
            0x56, 0x34, 0x12, 0, 0xef, 0xcd, 0xab, 255, 0x56, 0x34, 0x12, 0
        ]
    );
}
