//! Drive a disposable real Chrome session using only Rusty Robot input/capture.
//! macOS: `cargo run --release --example cua`; one JSON command per stdin line.
//! Coordinates are pixels in the latest screenshot, identified by `frame`.
#[cfg(target_os = "macos")]
mod driver {
    use rusty_robot::{Bitmap, Point, Rect, Robot};
    use serde_json::{Value, json};
    use std::{
        fs,
        io::{BufRead, Write},
        path::PathBuf,
        process::{Child, Command, Stdio},
        thread,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    struct Session {
        child: Child,
        region: Rect,
        cursor: Point,
        frame: Option<Bitmap>,
        sequence: usize,
        output: PathBuf,
        trace: fs::File,
        _profile: tempfile::TempDir,
    }

    impl Session {
        fn application(&self, activate: bool) -> Result<Value> {
            let target = Robot
                .windows()?
                .into_iter()
                .find(|w| w.process_id == self.child.id())
                .ok_or("Owned Chrome window is unavailable")?;
            if activate {
                target.activate()?;
                rusty_robot::observation::wait_until(
                    Default::default(),
                    || target.refresh(),
                    |w| w.active,
                    || false,
                )
                .map_err(|e| format!("Chrome activation: {:?}", e.reason))?;
            }
            Ok(
                json!({"pid":target.process_id,"windowId":target.id,"active":target.refresh()?.active}),
            )
        }

        fn start() -> Result<Self> {
            if Robot.get_accessibility_permission()? == Some(false)
                || Robot.get_screen_capture_permission()? == Some(false)
            {
                return Err("Accessibility and capture permissions are required".into());
            }
            let displays = Robot.get_displays()?;
            eprintln!("Displays: {displays:?}");
            let display = displays
                .iter()
                .find(|d| d.is_main)
                .ok_or("No main display")?;
            let cursor = Robot.get_mouse_pos()?;
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let run = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
            let output = std::env::var_os("RUSTY_ROBOT_CUA_OUTPUT")
                .map(PathBuf::from)
                .unwrap_or_else(|| root.join("target/cua-chrome").join(run.to_string()));
            fs::create_dir_all(&output)?;
            let trace = fs::File::create_new(output.join("trace.jsonl"))?;
            let profile = tempfile::Builder::new()
                .prefix("rusty-robot-chrome-")
                .tempdir()?;
            let child =
                Command::new("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
                    .arg(format!("--user-data-dir={}", profile.path().display()))
                    .args([
                        "--no-first-run",
                        "--no-default-browser-check",
                        "--disable-background-mode",
                        "--window-position=0,30",
                        "--window-size=1900,1000",
                        "about:blank",
                    ])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()?;
            let session = Self {
                child,
                region: Rect {
                    x: display.x,
                    y: display.y,
                    width: display.width,
                    height: display.height,
                },
                cursor,
                frame: None,
                sequence: 0,
                output,
                trace,
                _profile: profile,
            };
            thread::sleep(Duration::from_secs(2));
            eprintln!("Chrome: {}", session.application(true)?);
            Robot.set_mouse_delay(10)?;
            Robot.set_keyboard_delay(5)?;
            Ok(session)
        }

        fn point(&self, command: &Value, x: &str, y: &str) -> Result<Point> {
            let frame = self.frame.as_ref().ok_or("Capture before input")?;
            if command["frame"].as_u64() != Some(self.sequence as u64) {
                return Err("Stale or missing frame ID; use the latest screenshot".into());
            }
            let point = Point {
                x: command[x].as_i64().ok_or("Expected integer x")?,
                y: command[y].as_i64().ok_or("Expected integer y")?,
            };
            if point.x < 0
                || point.y < 0
                || point.x as u64 >= frame.width as u64
                || point.y as u64 >= frame.height as u64
            {
                return Err("Point is outside the screenshot".into());
            }
            Ok(frame.to_screen_point(point, None)?)
        }

        fn capture(&mut self) -> Result<Value> {
            let start = Instant::now();
            let frame = Robot.capture(Some(self.region))?;
            let capture_ms = start.elapsed().as_secs_f64() * 1000.0;
            self.sequence += 1;
            let extension = if cfg!(feature = "png") { "png" } else { "bmp" };
            let path = self
                .output
                .join(format!("frame-{:03}.{extension}", self.sequence));
            let start = Instant::now();
            #[cfg(feature = "png")]
            fs::write(&path, rusty_robot::image::encode_png(&frame, true)?)?;
            #[cfg(not(feature = "png"))]
            frame.save(&path)?;
            let result = json!({"frame": self.sequence, "path": path,
            "width": frame.width, "height": frame.height,
            "screenX": frame.screen_x, "screenY": frame.screen_y,
            "scaleX": frame.scale_x, "scaleY": frame.scale_y,
            "captureMs": capture_ms, "saveMs": start.elapsed().as_secs_f64()*1000.0});
            self.frame = Some(frame);
            Ok(result)
        }

        fn execute(&mut self, command: &Value) -> Result<Value> {
            let action = command["action"].as_str().ok_or("Missing action")?;
            if action == "capture" {
                return self.capture();
            }
            if action == "observe" {
                return self.application(false);
            }
            if action == "focus" {
                return self.application(true);
            }
            if action == "benchmark" {
                let mut results = Vec::new();
                for (name, rect) in [
                    (
                        "pixel",
                        Some(Rect {
                            width: 1,
                            height: 1,
                            ..self.region
                        }),
                    ),
                    ("display_rect", Some(self.region)),
                    ("display", None),
                ] {
                    let mut samples = Vec::new();
                    for _ in 0..12 {
                        let start = Instant::now();
                        let bitmap = Robot.capture(rect)?;
                        std::hint::black_box(bitmap);
                        samples.push(start.elapsed().as_secs_f64() * 1000.0);
                    }
                    results.push(json!({"case": name, "milliseconds": samples}));
                }
                return Ok(json!(results));
            }
            let state = self.application(false)?;
            if state["active"] != true {
                return Err(
                    "Owned Chrome is not focused; input refused. Use focus then capture.".into(),
                );
            }
            let start = Instant::now();
            match action {
                "click" => {
                    let p = self.point(command, "x", "y")?;
                    let button = command["button"].as_str().unwrap_or("left");
                    if !matches!(button, "left" | "right" | "middle") {
                        return Err("Expected left, right, or middle button".into());
                    }
                    Robot.move_mouse(p.x, p.y)?;
                    Robot
                        .mouse_click(Some(button), command["double"].as_bool().unwrap_or(false))?;
                }
                "type" => Robot.type_string(command["text"].as_str().ok_or("Missing text")?)?,
                "key" => {
                    let modifiers: Vec<&str> = command["modifiers"]
                        .as_array()
                        .map(|v| {
                            v.iter()
                                .map(|s| s.as_str().ok_or("Expected modifier name"))
                                .collect()
                        })
                        .transpose()?
                        .unwrap_or_default();
                    Robot.key_tap(command["key"].as_str().ok_or("Missing key")?, &modifiers)?;
                }
                "scroll" => {
                    let p = self.point(command, "x", "y")?;
                    let dx = command["dx"].as_i64().ok_or("Missing dx")?;
                    let dy = command["dy"].as_i64().ok_or("Missing dy")?;
                    Robot.move_mouse(p.x, p.y)?;
                    Robot.scroll_mouse(dx, dy)?;
                }
                "drag" => {
                    let from = self.point(command, "x", "y")?;
                    let to = self.point(command, "toX", "toY")?;
                    Robot.input_session()?.drag_path(
                        rusty_robot::session::MouseButton::Left,
                        from,
                        (1..=24).map(|step| Point {
                            x: from.x + (to.x - from.x) * step / 24,
                            y: from.y + (to.y - from.y) * step / 24,
                        }),
                        || false,
                    )?;
                }
                _ => return Err("Unknown action".into()),
            }
            let action_ms = start.elapsed().as_secs_f64() * 1000.0;
            // Explicit settling assumption, not a guarantee of application completion.
            thread::sleep(Duration::from_millis(150));
            Ok(json!({"actionMs": action_ms, "capture": self.capture()?}))
        }
    }

    impl Drop for Session {
        fn drop(&mut self) {
            // Only drag holds an input, and it releases before returning even on error.
            let _ = Robot.move_mouse(self.cursor.x, self.cursor.y);
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    pub fn run() -> Result<()> {
        let mut session = Session::start()?;
        println!("{}", session.capture()?);
        for line in std::io::stdin().lock().lines() {
            let line = line?;
            let command: Value = match serde_json::from_str(&line) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", json!({"error": error.to_string()}));
                    continue;
                }
            };
            if command["action"] == "quit" {
                break;
            }
            let result = match session.execute(&command) {
                Ok(value) => value,
                Err(error) => json!({"error": error.to_string()}),
            };
            writeln!(
                session.trace,
                "{}",
                json!({"command": command, "result": result})
            )?;
            session.trace.flush()?;
            println!("{result}");
        }
        Ok(())
    }
}
#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    driver::run()
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This Chrome dogfood example currently requires macOS.");
}
