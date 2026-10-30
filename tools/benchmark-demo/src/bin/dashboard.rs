use rusty_robot::{Bitmap, Point};
use serde_json::{Value, json};
use std::{
    hint::black_box,
    io::{self, Write},
    path::Path,
    time::Instant,
};
fn boxes(points: Vec<Point>) -> Vec<[i64; 4]> {
    let mut out: Vec<[i64; 4]> = Vec::new();
    for p in points {
        if let Some(b) = out
            .iter_mut()
            .find(|b| p.y <= b[3] + 1 && p.x >= b[0] - 1 && p.x <= b[2] + 1)
        {
            b[0] = b[0].min(p.x);
            b[1] = b[1].min(p.y);
            b[2] = b[2].max(p.x);
            b[3] = b[3].max(p.y)
        } else {
            out.push([p.x, p.y, p.x, p.y]);
        }
    }
    out.into_iter()
        .map(|b| [b[0], b[1], b[2] - b[0] + 1, b[3] - b[1] + 1])
        .collect()
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(&args[1]);
    let record = args.get(2).is_some_and(|s| s == "record");
    let fixtures: Vec<Value> =
        serde_json::from_slice(&std::fs::read(root.join("fixtures.json")).unwrap()).unwrap();
    let images: Vec<Bitmap> = fixtures
        .iter()
        .map(|f| Bitmap {
            width: 1920,
            height: 1080,
            byte_width: 7680,
            bits_per_pixel: 32,
            bytes_per_pixel: 4,
            image: std::fs::read(root.join(format!("{}.bgra", f["name"].as_str().unwrap())))
                .unwrap(),
            screen_x: 0,
            screen_y: 0,
            scale_x: 1.0,
            scale_y: 1.0,
        })
        .collect();
    let run = |b: &Bitmap| {
        boxes(
            black_box(b)
                .find_colors("2eac68", Default::default())
                .unwrap(),
        )
    };
    for (image, f) in images.iter().zip(&fixtures) {
        assert_eq!(json!(run(image)), f["expected"]);
    }
    println!("{}", json!({"type":"ready"}));
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    assert_eq!(line.trim(), "go");
    let start = Instant::now();
    let mut results = Vec::new();
    for (i, image) in images.iter().enumerate() {
        let found = run(image);
        if record {
            println!(
                "{}",
                json!({"type":"progress","index":i,"boxes":found,"seconds":start.elapsed().as_secs_f64()})
            );
            io::stdout().flush().unwrap();
        }
        results.push(found);
    }
    let seconds = start.elapsed().as_secs_f64();
    for (found, f) in results.iter().zip(&fixtures) {
        assert_eq!(json!(found), f["expected"]);
    }
    println!(
        "{}",
        json!({"type":"complete","seconds":seconds,"screenshots":images.len(),"targets":results.iter().map(|r|r.len()).sum::<usize>(),"verified":true,"results":results})
    );
}
