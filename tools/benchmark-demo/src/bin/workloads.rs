use rusty_robot::Bitmap;
use serde_json::{Value, json};
use std::{
    hint::black_box,
    io::{self, Write},
    path::Path,
    time::Instant,
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(&args[1]);
    let case = args[2].as_str();
    assert!(matches!(case, "alerts" | "heatmap"));
    let record = args.get(3).is_some_and(|s| s == "record");
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
    enum Output {
        Alerts(Vec<[i64; 4]>),
        Heatmap(usize),
    }
    let run = |b: &Bitmap| -> Output {
        match case {
            "alerts" => Output::Alerts(
                black_box(b)
                    .find_color("d14343", Default::default())
                    .unwrap()
                    .map(|p| [p.x, p.y, 100, 28])
                    .into_iter()
                    .collect(),
            ),
            "heatmap" => Output::Heatmap(
                black_box(b)
                    .count_color("2eac68", Default::default())
                    .unwrap(),
            ),
            _ => unreachable!(),
        }
    };
    let display = |value: &Output| -> Value {
        match value {
            Output::Alerts(boxes) => json!(boxes),
            Output::Heatmap(pixels) => json!(pixels / 70),
        }
    };
    let verify = |value: &Output, fixture: &Value| match value {
        Output::Alerts(boxes) => assert_eq!(json!(boxes), fixture["expected"]),
        Output::Heatmap(pixels) => {
            assert_eq!(*pixels as u64, fixture["expected"].as_u64().unwrap() * 70)
        }
    };
    for (image, f) in images.iter().zip(&fixtures) {
        verify(&run(image), f);
    }
    println!("{}", json!({"type":"ready"}));
    io::stdout().flush().unwrap();
    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap();
    assert_eq!(line.trim(), "go");
    let start = Instant::now();
    let mut results = Vec::new();
    for (i, image) in images.iter().enumerate() {
        let result = run(image);
        if record {
            println!(
                "{}",
                json!({"type":"progress","index":i,"result":display(&result),"seconds":start.elapsed().as_secs_f64()})
            );
            io::stdout().flush().unwrap();
        }
        results.push(result);
    }
    let seconds = start.elapsed().as_secs_f64();
    for (result, f) in results.iter().zip(&fixtures) {
        verify(result, f);
    }
    let raw_pixel_counts: Vec<usize> = results
        .iter()
        .filter_map(|r| match r {
            Output::Heatmap(pixels) => Some(*pixels),
            _ => None,
        })
        .collect();
    let results: Vec<Value> = results.iter().map(display).collect();
    let targets: u64 = results
        .iter()
        .map(|r| {
            if case == "alerts" {
                r.as_array().unwrap().len() as u64
            } else {
                r.as_u64().unwrap()
            }
        })
        .sum();
    println!(
        "{}",
        json!({"type":"complete","case":case,"seconds":seconds,"screenshots":images.len(),"targets":targets,"verified":true,"results":results,"raw_pixel_counts":raw_pixel_counts})
    );
}
