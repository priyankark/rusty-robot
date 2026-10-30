use rusty_robot::{Rect, Robot};
use serde_json::{Value, json};
use std::{
    env, fs,
    io::{self, BufRead, Write},
    path::Path,
};
fn capture(request: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    let prefix = request["prefix"].as_str().ok_or("Missing output prefix")?;
    let rect = &request["rect"];
    let b = Robot.capture(Some(Rect {
        x: rect[0].as_i64().ok_or("x")?,
        y: rect[1].as_i64().ok_or("y")?,
        width: rect[2].as_u64().ok_or("width")? as usize,
        height: rect[3].as_u64().ok_or("height")? as usize,
    }))?;
    b.save(Path::new(&format!("{prefix}.png")))?;
    fs::write(format!("{prefix}.bgra"), &b.image)?;
    Ok(
        json!({"width":b.width,"height":b.height,"byte_width":b.byte_width,"bytes_per_pixel":b.bytes_per_pixel,"bits_per_pixel":b.bits_per_pixel,"screen_x":b.screen_x,"screen_y":b.screen_y,"scale_x":b.scale_x,"scale_y":b.scale_y}),
    )
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(Robot.get_screen_capture_permission()?, Some(true));
    let args: Vec<_> = env::args().collect();
    if args.get(1).is_some_and(|s| s == "--stream") {
        for line in io::stdin().lock().lines() {
            let request: Value = serde_json::from_str(&line?)?;
            let response =
                capture(&request).unwrap_or_else(|error| json!({"error":error.to_string()}));
            println!("{response}");
            io::stdout().flush()?;
        }
    } else {
        assert_eq!(args.len(), 6, "app-capture OUTPUT_PREFIX X Y WIDTH HEIGHT");
        let rect: Vec<i64> = args[2..]
            .iter()
            .map(|v| v.parse())
            .collect::<Result<_, _>>()?;
        println!("{}", capture(&json!({"prefix":args[1],"rect":rect}))?);
    }
    Ok(())
}
