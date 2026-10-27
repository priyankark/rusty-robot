//! Opt-in native workflows with independent Chromium observations and OS screenshots.
use rusty_robot::{Bitmap, Point, Rect, Robot};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

struct Desktop {
    child: Child,
    input: ChildStdin,
    output: Receiver<Value>,
    ready: Value,
    artifacts: PathBuf,
    cursor: Point,
}

impl Desktop {
    fn start() -> Self {
        let cursor = Robot.get_mouse_pos().unwrap();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let resolved = Command::new("node")
            .args(["-p", "require('electron')"])
            .current_dir(root.join("tests/gui"))
            .output()
            .expect("Node and npm ci --prefix tests/gui required");
        assert!(resolved.status.success());
        let mut command = Command::new(String::from_utf8(resolved.stdout).unwrap().trim());
        if cfg!(target_os = "linux") {
            command.arg("--no-sandbox");
        }
        let mut child = command
            .arg(root.join("tests/gui/verification/main.cjs"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, output) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Ok(value) = serde_json::from_str::<Value>(&line)
                    && tx.send(value).is_err()
                {
                    break;
                }
            }
        });
        let run = if std::env::var_os("RUSTY_ROBOT_QA_SECONDARY").is_some() {
            "secondary"
        } else {
            "primary"
        };
        let artifacts = std::env::var_os("RUSTY_ROBOT_QA_ARTIFACTS")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("target/desktop-qa").join(run));
        fs::create_dir_all(&artifacts).unwrap();
        let mut desktop = Self {
            child,
            input,
            output,
            ready: Value::Null,
            artifacts,
            cursor,
        };
        desktop.ready = desktop.receive();
        assert_eq!(desktop.ready["ready"], true, "{}", desktop.ready);
        fs::write(
            desktop.artifacts.join("geometry.json"),
            desktop.ready.to_string(),
        )
        .unwrap();
        Robot.set_mouse_delay(15).unwrap();
        Robot.set_keyboard_delay(5).unwrap();
        let marker = desktop.center("marker");
        let deadline = Instant::now() + Duration::from_secs(10);
        desktop.bring_to_front();
        loop {
            desktop.request(json!({"focus": true}));
            // get_pixel_color intentionally follows RobotJS's main-screen-only
            // contract; signed secondary coordinates use capture instead.
            let pixel = Robot
                .capture(Some(Rect {
                    x: marker.x,
                    y: marker.y,
                    width: 1,
                    height: 1,
                }))
                .unwrap();
            if pixel.color_at(0, 0).unwrap() == "c0ff33" {
                break;
            }
            assert!(Instant::now() < deadline, "QA window not visible");
            thread::sleep(Duration::from_millis(100));
        }
        desktop.evidence("00-ready");
        desktop
    }
    fn receive(&self) -> Value {
        let value = self
            .output
            .recv_timeout(Duration::from_secs(15))
            .expect("QA response");
        assert!(value.get("error").is_none(), "{value}");
        value
    }
    fn bring_to_front(&self) {
        #[cfg(target_os = "macos")]
        {
            use objc2::{
                msg_send,
                rc::Retained,
                runtime::{AnyClass, AnyObject},
            };
            #[link(name = "AppKit", kind = "framework")]
            unsafe extern "C" {}
            // Activate only the disposable process owned by this fixture.
            unsafe {
                if let Some(class) = AnyClass::get(c"NSRunningApplication") {
                    let app: Option<Retained<AnyObject>> = msg_send![class,runningApplicationWithProcessIdentifier:self.child.id() as i32];
                    if let Some(app) = app {
                        let _: bool = msg_send![&app,activateWithOptions:3usize];
                    }
                }
            }
        }
    }
    fn request(&mut self, command: Value) -> Value {
        writeln!(self.input, "{command}").unwrap();
        self.input.flush().unwrap();
        self.receive()
    }
    fn state(&mut self) -> Value {
        thread::sleep(Duration::from_millis(100));
        self.request(json!({}))
    }
    fn wait(&mut self, label: &str, predicate: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let state = self.state();
            if predicate(&state) {
                return state;
            }
            assert!(
                Instant::now() < deadline,
                "{label}: {}",
                json!({
                    "counts": state["counts"], "scroll": state["scroll"],
                    "nativeFocus": state["nativeFocus"], "rendererFocus": state["rendererFocus"],
                    "textLength": state["text"].as_str().unwrap_or_default().chars().count(),
                    "textPrefix": state["text"].as_str().unwrap_or_default().chars().take(100).collect::<String>(),
                    "lastEvents": state["events"].as_array().unwrap().iter().rev().take(5).collect::<Vec<_>>()
                })
            );
        }
    }
    fn region(&self, id: &str) -> Rect {
        let r = &self.ready["elements"][id];
        Rect {
            x: self.ready["bounds"]["x"].as_i64().unwrap()
                + r["x"].as_f64().unwrap().round() as i64,
            y: self.ready["bounds"]["y"].as_i64().unwrap()
                + r["y"].as_f64().unwrap().round() as i64,
            width: r["width"].as_f64().unwrap().round() as usize,
            height: r["height"].as_f64().unwrap().round() as usize,
        }
    }
    fn center(&self, id: &str) -> Point {
        let r = self.region(id);
        Point {
            x: r.x + r.width as i64 / 2,
            y: r.y + r.height as i64 / 2,
        }
    }
    fn focus(&self, id: &str) {
        let p = self.center(id);
        Robot.move_mouse(p.x, p.y).unwrap();
        Robot.mouse_click(None, false).unwrap();
    }
    fn capture(&self) -> Bitmap {
        let b = &self.ready["bounds"];
        Robot
            .capture(Some(Rect {
                x: b["x"].as_i64().unwrap(),
                y: b["y"].as_i64().unwrap(),
                width: b["width"].as_u64().unwrap() as usize,
                height: b["height"].as_u64().unwrap() as usize,
            }))
            .unwrap()
    }
    fn evidence(&mut self, label: &str) {
        let state = self.request(json!({"phase": label}));
        fs::write(
            self.artifacts.join(format!("{label}.json")),
            serde_json::to_vec_pretty(&state).unwrap(),
        )
        .unwrap();
        thread::sleep(Duration::from_millis(150));
        self.capture()
            .save(&self.artifacts.join(format!("{label}-robot.bmp")))
            .unwrap();
        #[cfg(target_os = "macos")]
        {
            // Independent OS screenshot, never generated from Robot's bitmap.
            let b = &self.ready["bounds"];
            let region = format!("{},{},{},{}", b["x"], b["y"], b["width"], b["height"]);
            assert!(
                Command::new("/usr/sbin/screencapture")
                    .args(["-x", "-R", &region])
                    .arg(self.artifacts.join(format!("{label}-os.png")))
                    .status()
                    .unwrap()
                    .success()
            );
        }
        println!("Checkpoint {label}; evidence: {}", self.artifacts.display());
    }
}
impl Drop for Desktop {
    fn drop(&mut self) {
        // Release any input even after a failed assertion, then restore the cursor.
        for key in ["shift", "control", "alt", "command"] {
            let _ = Robot.key_toggle(key, "up", &[]);
        }
        for button in ["left", "right", "middle"] {
            let _ = Robot.mouse_toggle("up", Some(button));
        }
        if std::thread::panicking() && !self.ready.is_null() {
            let _ =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.evidence("failure")));
        }
        #[cfg(unix)]
        unsafe {
            libc::kill(self.child.id() as i32, libc::SIGTERM);
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while matches!(self.child.try_wait(), Ok(None)) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(25));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = Robot.move_mouse(self.cursor.x, self.cursor.y);
        let _ = Robot.set_mouse_delay(10);
        let _ = Robot.set_keyboard_delay(10);
    }
}

#[test]
#[ignore = "moves the real pointer and types into an owned Electron QA window"]
fn visible_desktop_workflows() {
    let mut ui = Desktop::start();
    // 1. Each button must produce the browser's actual native event, including dblclick.
    ui.focus("clicks");
    ui.wait("single click", |s| s["counts"]["click"] == 1);
    assert_eq!(Robot.get_mouse_pos().unwrap(), ui.center("clicks"));
    Robot.mouse_click(None, true).unwrap();
    assert_eq!(Robot.get_mouse_pos().unwrap(), ui.center("clicks"));
    ui.wait("double click", |s| {
        s["counts"]["dblclick"] == 1 && s["counts"]["click"] == 3
    });
    Robot.mouse_click(Some("right"), false).unwrap();
    ui.wait("right click", |s| s["counts"]["contextmenu"] == 1);
    Robot.mouse_click(Some("middle"), false).unwrap();
    ui.wait("middle click", |s| s["counts"]["middle"] == 1);
    ui.evidence("01-mouse-buttons");

    // 2. Smooth movement must reach the requested screen coordinate.
    let start = ui.center("drag");
    Robot
        .move_mouse_smooth(start.x, start.y, Some(1.0))
        .unwrap();
    assert_eq!(Robot.get_mouse_pos().unwrap(), start);
    let end = ui.center("drop");
    Robot.mouse_toggle("down", None).unwrap();
    ui.wait("held mouse button state", |s| {
        s["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["id"] == "drag" && e["type"] == "mousedown" && e["buttons"] == 1)
    });
    for n in 1..=24 {
        Robot
            .drag_mouse(
                start.x + (end.x - start.x) * n / 24,
                start.y + (end.y - start.y) * n / 24,
                None,
            )
            .unwrap();
    }
    Robot.mouse_toggle("up", None).unwrap();
    ui.wait("drag and drop", |s| s["counts"]["drops"] == 1);
    ui.evidence("02-drag-and-drop");

    // 3. Both axes, their inverse directions, and no cross-axis movement.
    ui.focus("scroll");
    Robot.scroll_mouse(0, -100).unwrap();
    let vertical = ui.wait("vertical scroll", |s| {
        s["scroll"]["y"].as_i64().unwrap() > 0
    });
    assert_eq!(vertical["scroll"]["x"], 0);
    Robot.scroll_mouse(-100, 0).unwrap();
    let horizontal = ui.wait("horizontal scroll", |s| {
        s["scroll"]["x"].as_i64().unwrap() > 0
    });
    assert_eq!(horizontal["scroll"]["y"], vertical["scroll"]["y"]);
    ui.evidence("03-scroll-both-axes");
    Robot.scroll_mouse(100, 100).unwrap();
    ui.wait("reverse both axes", |s| {
        s["scroll"]["x"] == 0 && s["scroll"]["y"] == 0
    });

    // 4. UTF-16 surrogate pairs, combining sequences, ZWJ, RTL, CJK and newline.
    ui.focus("editor");
    let unicode = "Rusty Robot: café e\u{301} 中文 日本語 مرحبا 👩‍💻 🚀\nSecond line: !@#$%^&*()[]{}";
    Robot.type_string(unicode).unwrap();
    ui.wait("Unicode and multiline typing", |s| s["text"] == unicode);
    Robot.unicode_tap('\n').unwrap();
    Robot.type_string_delayed("Paced\ntext", 60_000).unwrap();
    let multiline = format!("{unicode}\nPaced\ntext");
    ui.wait("Unicode tap and paced newlines", |s| s["text"] == multiline);
    ui.evidence("04-unicode-multiline");

    // 5. Native selection/editing plus modifier release, using rendered text as oracle.
    let command = if cfg!(target_os = "macos") {
        "command"
    } else {
        "control"
    };
    Robot.key_tap("a", &[command]).unwrap();
    Robot.type_string("abcd").unwrap();
    Robot.key_tap("left", &["shift"]).unwrap();
    Robot.type_string("Z").unwrap();
    Robot.key_toggle("shift", "down", &[]).unwrap();
    Robot.key_tap("a", &[]).unwrap();
    Robot.key_toggle("shift", "up", &[]).unwrap();
    Robot.key_tap("a", &[]).unwrap();
    ui.wait("selection and held modifier", |s| s["text"] == "abcZAa");
    Robot.key_tap("z", &[command]).unwrap();
    ui.wait("undo", |s| s["text"] != "abcZAa");
    ui.evidence("05-editing-shortcuts");

    // 6. Pattern captured from real screen, relocated in a larger capture, clicked natively.
    let marker = ui.region("marker");
    let needle = Robot.capture(Some(marker)).unwrap();
    assert_eq!(needle.color_at(0, 0).unwrap(), "c0ff33");
    assert_eq!(needle.color_at(20, 20).unwrap(), "e94079");
    let screen = ui.capture();
    let found = screen
        .find_image(&needle, Default::default())
        .unwrap()
        .unwrap();
    assert_eq!(
        screen.to_screen_point(found, None).unwrap(),
        Point {
            x: marker.x,
            y: marker.y
        }
    );
    let clicked = screen
        .click_image(&mut Robot, &needle, Default::default(), None, false)
        .unwrap();
    assert_eq!(clicked, Some(found));
    ui.wait("image-based native click", |s| s["counts"]["marker"] == 1);
    needle.save(&ui.artifacts.join("needle.bmp")).unwrap();
    let decoded = rusty_robot::image::load(&ui.artifacts.join("needle.bmp")).unwrap();
    assert_eq!(
        screen.find_image(&decoded, Default::default()).unwrap(),
        Some(found)
    );
    ui.evidence("06-image-search-and-click");

    // 7. Repeated fast events: verify observed counts and exact Unicode text, not API success.
    ui.focus("clicks");
    let before = ui.wait("focus click", |s| s["counts"]["click"] == 4)["counts"]["click"]
        .as_u64()
        .unwrap();
    Robot.set_mouse_delay(2).unwrap();
    for _ in 0..200 {
        Robot.mouse_click(None, false).unwrap();
    }
    ui.wait("200 rapid clicks", |s| {
        s["counts"]["click"].as_u64().unwrap() == before + 200
    });
    Robot.set_mouse_delay(15).unwrap();
    ui.focus("editor");
    Robot.key_tap("a", &[command]).unwrap();
    Robot.set_keyboard_delay(0).unwrap();
    let stress = "abc 中文 🚀 e\u{301} | ".repeat(100);
    Robot.type_string(&stress).unwrap();
    ui.wait("rapid Unicode typing", |s| s["text"] == stress);
    Robot.set_keyboard_delay(5).unwrap();
    ui.evidence("07-input-stress");

    // 8. Paced typing, tiny crops, repeated capture, and invalid input without desktop effects.
    Robot.key_tap("a", &[command]).unwrap();
    Robot
        .type_string_delayed("All desktop workflows passed ✓", 6000)
        .unwrap();
    ui.wait("paced Unicode typing", |s| {
        s["text"] == "All desktop workflows passed ✓"
    });
    let position = Robot.get_mouse_pos().unwrap();
    assert!(Robot.move_mouse_smooth(0, 0, Some(f64::NAN)).is_err());
    assert!(Robot.mouse_click(Some("invalid"), false).is_err());
    assert!(Robot.key_tap("a", &["invalid"]).is_err());
    assert_eq!(Robot.get_mouse_pos().unwrap(), position);
    for _ in 0..20 {
        let pixel = Robot
            .capture(Some(Rect {
                x: marker.x,
                y: marker.y,
                width: 1,
                height: 1,
            }))
            .unwrap();
        assert_eq!(pixel.color_at(0, 0).unwrap(), "c0ff33");
        assert_eq!(
            pixel.to_screen_point(Point::default(), None).unwrap(),
            Point {
                x: marker.x,
                y: marker.y
            }
        );
    }
    let observed = ui.state();
    for event in observed["events"].as_array().unwrap() {
        if let Some(trusted) = event.get("trusted") {
            assert_eq!(trusted, true, "synthetic event: {event}");
        }
    }
    ui.evidence("08-all-workflows-passed");
}
