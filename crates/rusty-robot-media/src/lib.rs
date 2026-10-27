//! Capture, bounded image decoding, SIMD/threaded resizing, frame hashing and encoding.
//! Transport and scheduling stay with the host; all pixel work happens here.
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer, images::Image};
use image::{ExtendedColorType, ImageEncoder, ImageReader};
use rusty_robot::{Bitmap, Robot};
use std::{io::Cursor, sync::OnceLock, time::Instant};

pub type Result<T> = std::result::Result<T, String>;
const MAX_PIXELS: u64 = 64 * 1024 * 1024;
const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Jpeg,
    Png,
}
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub width: Option<u32>,
    pub quality: u8,
    pub format: Format,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            width: Some(1440),
            quality: 85,
            format: Format::Jpeg,
        }
    }
}
#[derive(Debug)]
pub struct Frame {
    pub capture_ms: f64,
    pub convert_ms: f64,
    pub resize_ms: f64,
    pub hash_ms: f64,
    pub encode_ms: f64,
    pub data: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub fingerprint: String,
}
fn dimensions(width: u32, height: u32) -> Result<usize> {
    let pixels = u64::from(width) * u64::from(height);
    if width == 0 || height == 0 || width > 16384 || height > 16384 || pixels > MAX_PIXELS {
        return Err("Image dimensions exceed the supported pixel budget".into());
    }
    Ok(pixels as usize * 3)
}
fn options(value: Options) -> Result<()> {
    if !(1..=100).contains(&value.quality) || value.width.is_some_and(|v| v == 0 || v > 16384) {
        return Err("Width must be 1..16384 and quality 1..100".into());
    }
    Ok(())
}
fn buffer(length: usize) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    data.try_reserve_exact(length).map_err(|e| e.to_string())?;
    data.resize(length, 0);
    Ok(data)
}
fn resize(
    rgb: Vec<u8>,
    width: u32,
    height: u32,
    target_width: u32,
    target_height: u32,
) -> Result<Vec<u8>> {
    let length = dimensions(target_width, target_height)?;
    if width == target_width && height == target_height {
        return Ok(rgb);
    }
    let source =
        Image::from_vec_u8(width, height, rgb, PixelType::U8x3).map_err(|e| e.to_string())?;
    let mut target = Image::from_vec_u8(
        target_width,
        target_height,
        buffer(length)?,
        PixelType::U8x3,
    )
    .map_err(|e| e.to_string())?;
    // One shared four-thread pool bounds total resize parallelism across captures.
    // Node hosts additionally bound outstanding capture tasks to two.
    static POOL: OnceLock<std::result::Result<rayon::ThreadPool, String>> = OnceLock::new();
    let pool = POOL
        .get_or_init(|| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(
                    std::thread::available_parallelism()
                        .map_or(1, usize::from)
                        .min(4),
                )
                .thread_name(|i| format!("rusty-frame-{i}"))
                .build()
                .map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(Clone::clone)?;
    pool.install(|| {
        Resizer::new().resize(
            &source,
            &mut target,
            &ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Bilinear)),
        )
    })
    .map_err(|e| e.to_string())?;
    Ok(target.into_vec())
}
fn encode(rgb: &[u8], width: u32, height: u32, opts: Options) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    match opts.format {
        Format::Jpeg => {
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, opts.quality)
                .encode(rgb, width, height, ExtendedColorType::Rgb8)
                .map_err(|e| e.to_string())?
        }
        Format::Png => image::codecs::png::PngEncoder::new_with_quality(
            &mut encoded,
            image::codecs::png::CompressionType::Fast,
            image::codecs::png::FilterType::Adaptive,
        )
        .write_image(rgb, width, height, ExtendedColorType::Rgb8)
        .map_err(|e| e.to_string())?,
    }
    Ok(encoded)
}
fn process(
    rgb: Vec<u8>,
    source: (u32, u32),
    target: (u32, u32),
    opts: Options,
    previous: Option<&str>,
    force: bool,
) -> Result<Frame> {
    let (width, height) = target;
    let start = Instant::now();
    let rgb = resize(rgb, source.0, source.1, width, height)?;
    let resize_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let mut hash = blake3::Hasher::new();
    hash.update(&width.to_le_bytes());
    hash.update(&height.to_le_bytes());
    hash.update(&[
        opts.quality,
        if opts.format == Format::Jpeg { 0 } else { 1 },
    ]);
    hash.update(&rgb);
    let fingerprint = hash.finalize().to_hex().to_string();
    let hash_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let data = if !force && previous == Some(fingerprint.as_str()) {
        None
    } else {
        Some(encode(&rgb, width, height, opts)?)
    };
    let encode_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(Frame {
        capture_ms: 0.0,
        convert_ms: 0.0,
        resize_ms,
        hash_ms,
        encode_ms,
        data,
        width,
        height,
        source_width: source.0,
        source_height: source.1,
        fingerprint,
    })
}
/// Convert validated BGR/BGRA rows. Padding and alpha never enter visible-frame hashes.
pub fn process_bitmap(
    bitmap: &Bitmap,
    opts: Options,
    previous: Option<&str>,
    force: bool,
) -> Result<Frame> {
    options(opts)?;
    bitmap.validate().map_err(|e| e.to_string())?;
    let width = u32::try_from(bitmap.width).map_err(|e| e.to_string())?;
    let height = u32::try_from(bitmap.height).map_err(|e| e.to_string())?;
    let start = Instant::now();
    let mut rgb = buffer(dimensions(width, height)?)?;
    for (row, output) in bitmap
        .image
        .chunks_exact(bitmap.byte_width)
        .take(bitmap.height)
        .zip(rgb.chunks_exact_mut(bitmap.width * 3))
    {
        for (pixel, out) in row[..bitmap.width * bitmap.bytes_per_pixel as usize]
            .chunks_exact(bitmap.bytes_per_pixel as usize)
            .zip(output.as_chunks_mut::<3>().0)
        {
            out.copy_from_slice(&[pixel[2], pixel[1], pixel[0]]);
        }
    }
    let target_width = opts.width.unwrap_or(width).min(width);
    let target_height =
        (u64::from(height) * u64::from(target_width) / u64::from(width)).max(1) as u32;
    let convert_ms = start.elapsed().as_secs_f64() * 1000.0;
    let mut frame = process(
        rgb,
        (width, height),
        (target_width, target_height),
        opts,
        previous,
        force,
    )?;
    frame.convert_ms = convert_ms;
    Ok(frame)
}
/// Capture the main display and encode it without temporary files or subprocesses.
pub fn capture(mut opts: Options, previous: Option<&str>, force: bool) -> Result<Frame> {
    options(opts)?;
    let start = Instant::now();
    let bitmap = Robot.capture(None).map_err(|e| e.to_string())?;
    let capture_ms = start.elapsed().as_secs_f64() * 1000.0;
    // Requested preview widths are logical widths, matching remote input scaling.
    if let Some(width) = opts.width {
        let logical = (bitmap.width as f64 / bitmap.scale_x).round() as u32;
        opts.width = Some(width.min(logical.max(1)));
    }
    let mut frame = process_bitmap(&bitmap, opts, previous, force)?;
    frame.capture_ms = capture_ms;
    Ok(frame)
}
/// Decode JPEG/PNG with explicit dimension/allocation limits, resize, then encode.
pub fn resize_encoded(
    input: &[u8],
    width: u32,
    height: Option<u32>,
    opts: Options,
) -> Result<Frame> {
    options(opts)?;
    if input.len() > MAX_INPUT_BYTES {
        return Err("Encoded input exceeds 64 MiB".into());
    }
    dimensions(width, height.unwrap_or(1))?;
    let mut reader = ImageReader::new(Cursor::new(input))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Jpeg | image::ImageFormat::Png)
    ) {
        return Err("Only JPEG and PNG encoded input is supported".into());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(MAX_PIXELS * 4);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|e| e.to_string())?.into_rgb8();
    let source = decoded.dimensions();
    dimensions(source.0, source.1)?;
    let height = height.unwrap_or_else(|| {
        (u64::from(source.1) * u64::from(width) / u64::from(source.0)).max(1) as u32
    });
    process(
        decoded.into_raw(),
        source,
        (width, height),
        opts,
        None,
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn bitmap(bpp: u16, padding: usize, alpha: u8) -> Bitmap {
        let (width, height) = (8, 4);
        let stride = width * bpp as usize + padding;
        let mut image = vec![0x33; stride * height];
        for row in image.chunks_exact_mut(stride) {
            for pixel in row[..width * bpp as usize].chunks_exact_mut(bpp as usize) {
                pixel[..3].copy_from_slice(&[30, 60, 180]);
                if bpp == 4 {
                    pixel[3] = alpha;
                }
            }
        }
        Bitmap {
            width,
            height,
            byte_width: stride,
            bytes_per_pixel: bpp,
            bits_per_pixel: bpp * 8,
            image,
            screen_x: -100,
            screen_y: 0,
            scale_x: 1.0,
            scale_y: 1.0,
        }
    }
    #[test]
    fn png_channels_stride_alpha_dimensions_and_hash_are_consistent() {
        let opts = Options {
            width: Some(4),
            format: Format::Png,
            ..Options::default()
        };
        let first = process_bitmap(&bitmap(3, 2, 0), opts, None, false).unwrap();
        let decoded = image::load_from_memory(first.data.as_ref().unwrap())
            .unwrap()
            .into_rgb8();
        assert_eq!(decoded.dimensions(), (4, 2));
        assert!(decoded.pixels().all(|p| p.0 == [180, 60, 30]));
        for alpha in [0, 128, 255] {
            let same = process_bitmap(&bitmap(4, 7, alpha), opts, Some(&first.fingerprint), false)
                .unwrap();
            assert!(same.data.is_none());
            assert_eq!(same.fingerprint, first.fingerprint);
        }
    }
    #[test]
    fn force_quality_format_and_pixels_invalidate_unchanged_frames() {
        let b = bitmap(4, 0, 255);
        let opts = Options::default();
        let first = process_bitmap(&b, opts, None, false).unwrap();
        assert!(
            process_bitmap(&b, opts, Some(&first.fingerprint), true)
                .unwrap()
                .data
                .is_some()
        );
        for changed in [
            Options {
                quality: 70,
                ..opts
            },
            Options {
                format: Format::Png,
                ..opts
            },
            Options {
                width: Some(4),
                ..opts
            },
        ] {
            assert!(
                process_bitmap(&b, changed, Some(&first.fingerprint), false)
                    .unwrap()
                    .data
                    .is_some()
            );
        }
        let mut changed = b.clone();
        changed.image[0] ^= 0xff;
        assert!(
            process_bitmap(&changed, opts, Some(&first.fingerprint), false)
                .unwrap()
                .data
                .is_some()
        );
    }
    #[test]
    fn jpeg_is_decodable_and_encoded_resize_preserves_aspect_ratio() {
        let frame = process_bitmap(&bitmap(4, 3, 255), Options::default(), None, true).unwrap();
        let decoded = image::load_from_memory(frame.data.as_ref().unwrap())
            .unwrap()
            .into_rgb8();
        assert_eq!(decoded.dimensions(), (8, 4));
        for p in decoded.pixels() {
            for (actual, expected) in p.0.into_iter().zip([180u8, 60, 30]) {
                assert!(actual.abs_diff(expected) <= 3);
            }
        }
        let resized = resize_encoded(
            frame.data.as_ref().unwrap(),
            6,
            None,
            Options {
                format: Format::Png,
                ..Options::default()
            },
        )
        .unwrap();
        assert_eq!((resized.width, resized.height), (6, 3));
    }
    #[test]
    fn malformed_inputs_and_excessive_dimensions_fail() {
        assert!(resize_encoded(b"not an image", 4, None, Options::default()).is_err());
        for width in [0, 16385, u32::MAX] {
            assert!(
                process_bitmap(
                    &bitmap(3, 0, 0),
                    Options {
                        width: Some(width),
                        ..Options::default()
                    },
                    None,
                    false
                )
                .is_err()
            );
        }
        for quality in [0, 101, 255] {
            assert!(
                process_bitmap(
                    &bitmap(3, 0, 0),
                    Options {
                        quality,
                        ..Options::default()
                    },
                    None,
                    false
                )
                .is_err()
            );
        }
        let mut bad = bitmap(4, 0, 0);
        bad.image.clear();
        assert!(process_bitmap(&bad, Options::default(), None, true).is_err());
        assert!(dimensions(16384, 16384).is_err());
    }
}
