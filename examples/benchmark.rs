//! Comparable public-API microbenchmarks; driven by scripts/benchmark.py.
use rusty_robot::{Bitmap, Point, Rect, Robot, SearchOptions, image};
use serde_json::json;
use std::{hint::black_box, path::Path, time::Instant};

fn bitmap(path: &Path, w: usize, h: usize) -> Bitmap {
    Bitmap {
        width: w,
        height: h,
        byte_width: w * 4,
        bits_per_pixel: 32,
        bytes_per_pixel: 4,
        image: std::fs::read(path).unwrap(),
        screen_x: 0,
        screen_y: 0,
        scale_x: 1.0,
        scale_y: 1.0,
    }
}
fn point(p: Option<Point>) -> u32 {
    p.map_or(0, |p| ((p.x + 1) * 4096 + p.y + 1) as u32)
}
fn batch(
    f: &mut dyn FnMut() -> rusty_robot::Result<u32>,
    n: usize,
) -> rusty_robot::Result<(f64, u32)> {
    let start = Instant::now();
    let mut checksum = 0u32;
    for _ in 0..n {
        checksum = checksum.wrapping_add(black_box(f()?));
    }
    black_box(checksum);
    Ok((start.elapsed().as_secs_f64() * 1e9, checksum))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(&args[1]);
    let name = &args[2];
    let canvas = bitmap(&root.join("canvas.bgra"), 1920, 1080);
    let needle = bitmap(&root.join("needle.bgra"), 32, 32);
    let absent = bitmap(&root.join("absent.bgra"), 32, 32);
    let dense = bitmap(&root.join("dense.bgra"), 256, 256);
    let tile = bitmap(&root.join("tile.bgra"), 4, 4);
    let bmp = root.join("source.bmp");
    let png = root.join("source.png");
    let out_bmp = root.join("rust-output.bmp");
    let out_png = root.join("rust-output.png");
    let mut f = || -> rusty_robot::Result<u32> {
        let canvas = black_box(&canvas);
        Ok(match name.as_str() {
            "color_at" => u32::from_str_radix(&canvas.color_at(123, 456)?, 16).unwrap(),
            "find_color_early" => point(canvas.find_color("000000", Default::default())?),
            "find_color_late" => point(canvas.find_color("ff00ff", Default::default())?),
            "find_color_absent" => point(canvas.find_color("00ffff", Default::default())?),
            "count_color_absent" => canvas.count_color("00ffff", Default::default())? as u32,
            "count_color_dense" => dense.count_color("334455", Default::default())? as u32,
            "find_colors_dense" => dense.find_colors("334455", Default::default())?.len() as u32,
            "find_image_late" => point(canvas.find_image(&needle, Default::default())?),
            "find_image_absent" => point(canvas.find_image(&absent, Default::default())?),
            "find_image_tolerant" => point(canvas.find_image(
                &needle,
                SearchOptions {
                    tolerance: 0.02,
                    ..Default::default()
                },
            )?),
            "count_image_dense" => dense.count_image(&tile, Default::default())? as u32,
            "find_images_dense" => dense.find_images(&tile, Default::default())?.len() as u32,
            "load_bmp" | "load_png" => {
                let image = image::load(if name == "load_bmp" { &bmp } else { &png })?;
                let size = image.width * image.height;
                black_box(&image);
                size as u32
            }
            "save_bmp" => {
                canvas.save(&out_bmp)?;
                0
            }
            "save_png" => {
                canvas.save(&out_png)?;
                0
            }
            "get_mouse_pos" => point(Some(Robot.get_mouse_pos()?)),
            "get_displays" => Robot.get_displays()?.len() as u32,
            "get_pixel_color" => u32::from_str_radix(&Robot.get_pixel_color(0, 0)?, 16).unwrap(),
            "capture_small" | "capture_full" => {
                let captured = Robot.capture(if name == "capture_small" {
                    Some(Rect {
                        x: 0,
                        y: 0,
                        width: 100,
                        height: 100,
                    })
                } else {
                    None
                })?;
                let size = captured.width * captured.height;
                black_box(&captured);
                size as u32
            }
            _ => panic!("unknown benchmark {name}"),
        })
    };
    let observed = f()?;
    let expected = match name.as_str() {
        "color_at" => Some(
            ((123 * 3 + 456 * 5) % 128) << 16
                | ((123 * 7 + 456 * 13) % 128) << 8
                | ((123 * 17 + 456 * 11) % 128),
        ),
        "find_color_early" => Some(4097),
        "find_color_late" | "find_image_late" | "find_image_tolerant" => Some(1857 * 4096 + 1017),
        "find_color_absent" | "count_color_absent" | "find_image_absent" | "save_bmp"
        | "save_png" => Some(0),
        "count_color_dense" | "find_colors_dense" => Some(256 * 256),
        "count_image_dense" | "find_images_dense" => Some(253 * 253),
        "load_bmp" | "load_png" => Some(1920 * 1080),
        "capture_small" => Some(100 * 100),
        _ => None,
    };
    if let Some(expected) = expected {
        assert_eq!(observed, expected, "incorrect result for {name}");
    }
    // Decode/save correctness is checked outside timed loops against canonical pixels.
    if name.starts_with("load_") || name.starts_with("save_") {
        let path = match name.as_str() {
            "load_bmp" => &bmp,
            "load_png" => &png,
            "save_bmp" => &out_bmp,
            _ => &out_png,
        };
        let loaded = image::load(path)?;
        assert_eq!((loaded.width, loaded.height), (1920, 1080));
        for (x, y) in [(0, 0), (123, 456), (1856, 1016), (1887, 1047), (1919, 1079)] {
            assert_eq!(loaded.color_at(x, y)?, canvas.color_at(x, y)?);
        }
    }
    for _ in 0..5 {
        black_box(f()?);
    }
    let mut n = 1;
    while batch(&mut f, n)?.0 < 40_000_000.0 && n < 262144 {
        n *= 2;
    }
    let mut samples = Vec::new();
    let mut checksum = 0;
    for _ in 0..7 {
        let (ns, c) = batch(&mut f, n)?;
        samples.push(ns / n as f64);
        checksum ^= c;
    }
    println!(
        "{}",
        json!({"language":"rust","case":name,"iterations":n,"ns_per_op":samples,"observed":observed,"checksum":checksum})
    );
    Ok(())
}
