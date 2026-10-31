use rusty_robot::{Rect, Robot, SearchOptions};
use serde_json::{Value, json};
use std::{
    io::{self, BufRead, Write},
    time::{Duration, Instant},
};
fn read_code(
    b: &rusty_robot::Bitmap,
    points: &[[i64; 2]],
) -> Result<Option<u64>, Box<dyn std::error::Error>> {
    if points.is_empty() {
        return Ok(None);
    }
    let mut code = 0;
    for (i, p) in points.iter().enumerate() {
        match b.color_at(p[0], p[1])?.as_str() {
            "ffffff" => code |= 1 << i,
            "000000" => {}
            other => return Err(format!("Invalid code pixel: {other}").into()),
        }
    }
    Ok(Some(code))
}
fn run(v: &Value) -> Result<Value, Box<dyn std::error::Error>> {
    if v["op"] == "probe" {
        return Ok(
            json!({"accessibility":Robot.get_accessibility_permission()?,"capture":Robot.get_screen_capture_permission()?,"displays":format!("{:?}",Robot.get_displays()?),"pointer":format!("{:?}",Robot.get_mouse_pos()?)}),
        );
    }
    let a: [i64; 4] = serde_json::from_value(v["rect"].clone())?;
    let rect = Rect {
        x: a[0],
        y: a[1],
        width: a[2] as usize,
        height: a[3] as usize,
    };
    let sample: [i64; 2] = serde_json::from_value(v["sample"].clone())?;
    let points: Vec<[i64; 2]> = serde_json::from_value(v["code_points"].clone())?;
    let before = v["before"].as_str().ok_or("before missing")?;
    let after = v["after"].as_str().ok_or("after missing")?;
    Robot.set_mouse_delay(v["delay"].as_u64().unwrap_or(0))?;
    let start = Instant::now();
    let b = Robot.capture(Some(rect))?;
    let capture_ms = start.elapsed().as_secs_f64() * 1000.;
    if b.width != rect.width || b.height != rect.height || b.scale_x != 1. || b.scale_y != 1. {
        return Err("Unexpected capture geometry".into());
    }
    let actual_before = b.color_at(sample[0], sample[1])?;
    if actual_before != before {
        return Err(format!("First state mismatch: {actual_before} != {before}").into());
    }
    let first_code = read_code(&b, &points)?;
    if first_code.is_some() && first_code != v["code"].as_u64() {
        return Err("First frame trial code mismatch".into());
    }
    let mut result = json!({"capture_ms":capture_ms,"width":b.width,"height":b.height,"scale_x":b.scale_x,"scale_y":b.scale_y,"first_color":actual_before,"first_code":first_code});
    if v["op"] == "loop" {
        let search = Instant::now();
        let p = b
            .find_color("ff00ff", SearchOptions::default())?
            .ok_or("Target missing")?;
        let q = b.to_screen_point(p, None)?;
        let find_ms = search.elapsed().as_secs_f64() * 1000.;
        let x = q.x + 10;
        let y = q.y + 10;
        if x < rect.x
            || y < rect.y
            || x >= rect.x + rect.width as i64
            || y >= rect.y + rect.height as i64
        {
            return Err("Click out of owned bounds".into());
        }
        let input = Instant::now();
        Robot.move_mouse(x, y)?;
        Robot.mouse_click(None, false)?;
        let input_ms = input.elapsed().as_secs_f64() * 1000.;
        let observe = Instant::now();
        let mut polls = 0;
        let actual_after;
        let last_code;
        let mut complete = true;
        loop {
            let fresh = Robot.capture(Some(rect))?;
            polls += 1;
            let color = fresh.color_at(sample[0], sample[1])?;
            let code = read_code(&fresh, &points)?;
            if color == after && code == v["code"].as_u64() {
                last_code = code;
                actual_after = color;
                break;
            }
            if start.elapsed() > Duration::from_secs(1) {
                actual_after = color;
                last_code = code;
                complete = false;
                break;
            }
        }
        let observe_ms = observe.elapsed().as_secs_f64() * 1000.;
        let total_ms = start.elapsed().as_secs_f64() * 1000.;
        result["loop"] = json!({"find_ms":find_ms,"input_ms":input_ms,"observe_ms":observe_ms,"total_ms":total_ms,"polls":polls,"click":[x,y],"last_color":actual_after,"complete":complete,"last_code":last_code});
    }
    Ok(result)
}
fn main() {
    for line in io::stdin().lock().lines() {
        let answer = (|| -> Result<Value, Box<dyn std::error::Error>> {
            run(&serde_json::from_str(&line?)?)
        })();
        println!(
            "{}",
            match answer {
                Ok(v) => json!({"ok":true,"result":v}),
                Err(e) => json!({"ok":false,"error":e.to_string()}),
            }
        );
        io::stdout().flush().unwrap();
    }
}
