use rusty_robot::{Bitmap, SearchOptions};
use serde_json::{Value, json};
use std::{
    hint::black_box,
    io::{self, Write},
    path::Path,
    time::Instant,
};
fn regions(image: &Bitmap, colors: &[String], roi: [usize; 4]) -> Vec<[usize; 4]> {
    let mut rows = vec![(usize::MAX, None::<usize>); image.height];
    let options = SearchOptions {
        x: Some(roi[0]),
        y: Some(roi[1]),
        width: Some(roi[2]),
        height: Some(roi[3]),
        tolerance: 0.0,
    };
    for color in colors {
        for p in black_box(image).find_colors(color, options).unwrap() {
            let row = &mut rows[p.y as usize];
            row.0 = row.0.min(p.x as usize);
            row.1 = Some(row.1.map_or(p.x as usize, |r| r.max(p.x as usize)));
        }
    }
    let mut out: Vec<[usize; 4]> = Vec::new();
    for (y, (left, right)) in rows.into_iter().enumerate() {
        let Some(right) = right else { continue };
        if let Some(last) = out.last_mut().filter(|last| last[1] + last[3] == y) {
            let end = (last[0] + last[2]).max(right + 1);
            last[0] = last[0].min(left);
            last[2] = end - last[0];
            last[3] += 1;
        } else {
            out.push([left, y, right - left + 1, 1]);
        }
    }
    out
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(&args[1]);
    let record = args.get(2).is_some_and(|s| s == "record");
    let fixtures: Vec<Value> =
        serde_json::from_slice(&std::fs::read(root.join("fixtures.json")).unwrap()).unwrap();
    let images: Vec<_> = fixtures
        .iter()
        .map(|f| Bitmap {
            width: f["width"].as_u64().unwrap() as usize,
            height: f["height"].as_u64().unwrap() as usize,
            byte_width: f["byte_width"].as_u64().unwrap() as usize,
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
    let colors: Vec<Vec<String>> = fixtures
        .iter()
        .map(|f| serde_json::from_value(f["colors"].clone()).unwrap())
        .collect();
    let rois: Vec<[usize; 4]> = fixtures
        .iter()
        .map(|f| serde_json::from_value(f["roi"].clone()).unwrap())
        .collect();
    let run = |i: usize| regions(&images[i], colors[i].as_slice(), rois[i]);
    for (i, f) in fixtures.iter().enumerate() {
        assert_eq!(json!(run(i)), f["expected"]);
    }
    println!("{}", json!({"type":"ready"}));
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    assert_eq!(line.trim(), "go");
    let start = Instant::now();
    let mut results = Vec::new();
    for i in 0..fixtures.len() {
        results.push(run(i));
        if record {
            println!(
                "{}",
                json!({"type":"progress","index":i,"result":results.last().unwrap(),"seconds":start.elapsed().as_secs_f64()})
            );
            io::stdout().flush().unwrap();
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    for (r, f) in results.iter().zip(&fixtures) {
        assert_eq!(json!(r), f["expected"]);
    }
    println!(
        "{}",
        json!({"type":"complete","seconds":seconds,"screenshots":fixtures.len(),"targets":results.iter().map(|r|r.len()).sum::<usize>(),"verified":true,"results":results})
    );
}
