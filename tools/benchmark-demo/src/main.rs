use rusty_robot::{Bitmap, image};
use serde_json::json;
use std::{
    hint::black_box,
    io::{self, Write},
    path::Path,
    time::Instant,
};
fn bitmap(root: &Path, name: &str, width: usize, height: usize) -> Bitmap {
    Bitmap {
        width,
        height,
        byte_width: width * 4,
        bits_per_pixel: 32,
        bytes_per_pixel: 4,
        image: std::fs::read(root.join(name)).unwrap(),
        screen_x: 0,
        screen_y: 0,
        scale_x: 1.0,
        scale_y: 1.0,
    }
}
fn emit(value: serde_json::Value) {
    println!("{value}");
    io::stdout().flush().unwrap();
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(&args[1]);
    let case = &args[2];
    let total: usize = args[3].parse().unwrap();
    assert!((100..=100_000).contains(&total));
    let canvas = bitmap(root, "canvas.bgra", 1920, 1080);
    let dense = bitmap(root, "dense.bgra", 256, 256);
    let needle = bitmap(root, "needle.bgra", 32, 32);
    let run = || -> usize {
        match case.as_str() {
            "find_colors_dense" => black_box(&dense)
                .find_colors("334455", Default::default())
                .unwrap()
                .len(),
            "find_color_absent" => usize::from(
                black_box(&canvas)
                    .find_color("00ffff", Default::default())
                    .unwrap()
                    .is_some(),
            ),
            "find_image_late" => {
                let p = black_box(&canvas)
                    .find_image(&needle, Default::default())
                    .unwrap()
                    .unwrap();
                usize::try_from((p.x + 1) * 4096 + p.y + 1).unwrap()
            }
            "load_bmp" => {
                let b = image::load(&root.join("source.bmp")).unwrap();
                assert_eq!(
                    b.color_at(123, 456).unwrap(),
                    canvas.color_at(123, 456).unwrap()
                );
                black_box(b.width * b.height)
            }
            _ => panic!("unknown demo"),
        }
    };
    let expected = match case.as_str() {
        "find_colors_dense" => 65536,
        "find_color_absent" => 0,
        "find_image_late" => 1857 * 4096 + 1017,
        "load_bmp" => 1920 * 1080,
        _ => unreachable!(),
    };
    for _ in 0..5 {
        assert_eq!(black_box(run()), expected);
    }
    emit(json!({"type":"ready"}));
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    assert_eq!(line.trim(), "go");
    let start = Instant::now();
    for i in 1..=total {
        assert_eq!(black_box(run()), expected);
        if i % (total / 100) == 0 || i == total {
            emit(
                json!({"type":"progress","done":i,"total":total,"seconds":start.elapsed().as_secs_f64(),"result":expected,"verified":true}),
            );
        }
    }
    emit(
        json!({"type":"complete","done":total,"total":total,"seconds":start.elapsed().as_secs_f64(),"result":expected,"verified":true}),
    );
}
