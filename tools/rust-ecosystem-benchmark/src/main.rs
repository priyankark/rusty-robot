//! Standalone macOS benchmark driver. JSON lines are only control/measurement IPC.
use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use rusty_robot::{Rect, Robot};
use serde_json::{Value, json};
use std::{
    hint::black_box,
    io::{self, BufRead, Write},
    time::Instant,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Driver {
    backend: String,
    enigo: Option<Enigo>,
    monitor: Option<xcap::Monitor>,
}
impl Driver {
    fn run(&mut self, v: &Value) -> Result<Value> {
        let op = v["op"].as_str().ok_or("op required")?;
        let x = v["x"].as_i64().unwrap_or(0);
        let y = v["y"].as_i64().unwrap_or(0);
        let text = v["text"].as_str().unwrap_or("");
        if op == "config" {
            let delay = v["delay_ms"].as_u64().ok_or("delay_ms required")?;
            Robot.set_keyboard_delay(delay)?;
            Robot.set_mouse_delay(delay)?;
            return Ok(json!({"ok":true}));
        }
        let start = Instant::now();
        let mut detail = json!({});
        if op == "capture" {
            let w = v["width"].as_u64().ok_or("width required")? as usize;
            let h = v["height"].as_u64().ok_or("height required")? as usize;
            let full = v["full"].as_bool().unwrap_or(false);
            let (raw_w, raw_h, api_ms, rgb) = if self.backend == "rusty-robot" {
                let b = Robot.capture(if full {
                    None
                } else {
                    Some(Rect {
                        x,
                        y,
                        width: w,
                        height: h,
                    })
                })?;
                let api = start.elapsed().as_secs_f64() * 1000.;
                if b.width != w || b.height != h {
                    return Err("unexpected Rusty Robot dimensions".into());
                }
                let mut rgb = Vec::with_capacity(w * h * 3);
                for row in 0..h {
                    for col in 0..w {
                        let i = row * b.byte_width + col * usize::from(b.bytes_per_pixel);
                        rgb.extend_from_slice(&[b.image[i + 2], b.image[i + 1], b.image[i]]);
                    }
                }
                (b.width, b.height, api, rgb)
            } else {
                let m = self.monitor.as_ref().ok_or("capture needs xcap backend")?;
                let b = if full {
                    m.capture_image()?
                } else {
                    m.capture_region(x.try_into()?, y.try_into()?, w.try_into()?, h.try_into()?)?
                };
                let api = start.elapsed().as_secs_f64() * 1000.;
                let (rw, rh) = (b.width() as usize, b.height() as usize);
                if rw < w || rh < h || rw % w != 0 || rh % h != 0 {
                    return Err("unexpected XCap scale".into());
                }
                let sx = rw / w;
                let sy = rh / h;
                let mut rgb = Vec::with_capacity(w * h * 3);
                for row in 0..h {
                    for col in 0..w {
                        let i = ((row * sy + sy / 2) * rw + col * sx + sx / 2) * 4;
                        rgb.extend_from_slice(&b.as_raw()[i..i + 3]);
                    }
                }
                (rw, rh, api, rgb)
            };
            // Stop the timed interval before correctness probes/JSON serialization.
            let normalized_ms = start.elapsed().as_secs_f64() * 1000.;
            let probes: Vec<Value> = v["probes"]
                .as_array()
                .ok_or("probes required")?
                .iter()
                .map(|p| {
                    let px = p[0].as_u64().unwrap() as usize;
                    let py = p[1].as_u64().unwrap() as usize;
                    let i = (py * w + px) * 3;
                    json!(&rgb[i..i + 3])
                })
                .collect();
            black_box(&rgb);
            return Ok(
                json!({"ok":true,"api_ms":api_ms,"normalized_ms":normalized_ms,"raw_width":raw_w,"raw_height":raw_h,"width":w,"height":h,"probes":probes}),
            );
        }
        if self.backend == "rusty-robot" {
            match op {
                "text" => Robot.type_string(text)?,
                "replace" => {
                    Robot.key_tap("a", &["command"])?;
                    Robot.type_string(text)?;
                }
                "enter" => Robot.key_tap("enter", &[])?,
                "select_delete" => {
                    Robot.key_tap("a", &["command"])?;
                    Robot.key_tap("backspace", &[])?;
                }
                "move_click" => {
                    Robot.move_mouse(x, y)?;
                    Robot.mouse_click(Some("left"), false)?;
                }
                "move" => Robot.move_mouse(x, y)?,
                "position" => {
                    for _ in 0..1000 {
                        black_box(Robot.get_mouse_pos()?);
                    }
                }
                "display_size" => {
                    for _ in 0..100 {
                        black_box(Robot.get_screen_size()?);
                    }
                }
                _ => return Err("unknown operation".into()),
            }
        } else {
            let e = self.enigo.as_mut().ok_or("input needs enigo backend")?;
            match op {
                "text" => e.text(text)?,
                "replace" | "select_delete" => {
                    e.key(Key::Meta, Direction::Press)?;
                    let result = e.key(Key::Unicode('a'), Direction::Click);
                    let release = e.key(Key::Meta, Direction::Release);
                    result?;
                    release?;
                    if op == "replace" {
                        e.text(text)?;
                    } else {
                        e.key(Key::Backspace, Direction::Click)?;
                    }
                }
                "enter" => e.key(Key::Return, Direction::Click)?,
                "move_click" => {
                    e.move_mouse(x.try_into()?, y.try_into()?, Coordinate::Abs)?;
                    e.button(Button::Left, Direction::Click)?;
                }
                "move" => e.move_mouse(x.try_into()?, y.try_into()?, Coordinate::Abs)?,
                "position" => {
                    for _ in 0..1000 {
                        black_box(e.location()?);
                    }
                }
                "display_size" => {
                    for _ in 0..100 {
                        black_box(e.main_display()?);
                    }
                }
                _ => return Err("unknown operation".into()),
            }
        }
        detail["api_ms"] = json!(start.elapsed().as_secs_f64() * 1000.);
        detail["ok"] = json!(true);
        Ok(detail)
    }
}
fn main() -> Result<()> {
    let backend = std::env::args().nth(1).ok_or("backend required")?;
    let enigo = if backend == "enigo" {
        Some(Enigo::new(&Settings {
            open_prompt_to_get_permissions: false,
            ..Settings::default()
        })?)
    } else {
        None
    };
    let monitor = if backend == "xcap" {
        Some(
            xcap::Monitor::all()?
                .into_iter()
                .find(|m| m.is_primary().unwrap_or(false))
                .ok_or("no primary monitor")?,
        )
    } else {
        None
    };
    let displays = Robot.get_displays()?;
    let main = displays
        .iter()
        .find(|d| d.is_main)
        .ok_or("no primary display")?;
    let p = Robot.get_mouse_pos()?;
    println!(
        "{}",
        json!({"ready":true,"backend":backend,"accessibility":Robot.get_accessibility_permission()?,"capture_permission":Robot.get_screen_capture_permission()?,"display":{"width":main.width,"height":main.height,"x":main.x,"y":main.y},"cursor":{"x":p.x,"y":p.y}})
    );
    io::stdout().flush()?;
    let mut driver = Driver {
        backend,
        enigo,
        monitor,
    };
    for line in io::stdin().lock().lines() {
        let v: Value = serde_json::from_str(&line?)?;
        let result = driver
            .run(&v)
            .unwrap_or_else(|e| json!({"ok":false,"error":e.to_string()}));
        println!("{result}");
        io::stdout().flush()?;
    }
    Ok(())
}
