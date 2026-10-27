//! A second GUI toolkit to verify that native input works beyond Chromium.
#![cfg(target_os = "macos")]
use rusty_robot::{Point, Rect, Robot};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

struct NativeTarget {
    child: Child,
    input: ChildStdin,
    output: Receiver<Value>,
    geometry: Value,
    artifacts: PathBuf,
    cursor: Point,
}
impl NativeTarget {
    fn start() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let artifacts = std::env::var_os("RUSTY_ROBOT_QA_ARTIFACTS")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("target/release-validation/appkit"));
        fs::create_dir_all(&artifacts).unwrap();
        let executable = artifacts.join("native-target");
        assert!(
            Command::new("swiftc")
                .arg(root.join("tests/gui/native-macos.swift"))
                .arg("-o")
                .arg(&executable)
                .status()
                .expect("Xcode command line tools required")
                .success()
        );
        let cursor = Robot.get_mouse_pos().unwrap();
        let mut child = Command::new(executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, output) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Ok(value) = serde_json::from_str(&line)
                    && tx.send(value).is_err()
                {
                    break;
                }
            }
        });
        let mut target = Self {
            child,
            input,
            output,
            geometry: Value::Null,
            artifacts,
            cursor,
        };
        target.geometry = target.receive();
        assert_eq!(target.geometry["ready"], true);
        Robot.set_mouse_delay(30).unwrap();
        Robot.set_keyboard_delay(20).unwrap();
        target
    }
    fn receive(&self) -> Value {
        self.output
            .recv_timeout(Duration::from_secs(10))
            .expect("native fixture response")
    }
    fn state(&mut self, command: Value) -> Value {
        thread::sleep(Duration::from_millis(100));
        writeln!(self.input, "{command}").unwrap();
        self.input.flush().unwrap();
        self.receive()
    }
    fn focus(&mut self, name: &str) {
        assert_eq!(
            self.state(json!({}))["active"],
            true,
            "native target lost focus"
        );
        let r = &self.geometry[name];
        Robot
            .move_mouse(
                r["x"].as_i64().unwrap() + r["width"].as_i64().unwrap() / 2,
                r["y"].as_i64().unwrap() + r["height"].as_i64().unwrap() / 2,
            )
            .unwrap();
        Robot.mouse_click(None, false).unwrap();
    }
    fn text(&mut self, expected: &str) {
        let state = self.state(json!({}));
        assert_eq!(state["text"], expected, "native editor state: {state}");
    }
    fn evidence(&mut self, phase: &str) {
        let state = self.state(json!({"phase":phase}));
        fs::write(
            self.artifacts.join(format!("{phase}.json")),
            serde_json::to_vec_pretty(&state).unwrap(),
        )
        .unwrap();
        let r = &self.geometry["window"];
        let rect = Rect {
            x: r["x"].as_i64().unwrap(),
            y: r["y"].as_i64().unwrap(),
            width: r["width"].as_u64().unwrap() as usize,
            height: r["height"].as_u64().unwrap() as usize,
        };
        Robot
            .capture(Some(rect))
            .unwrap()
            .save(&self.artifacts.join(format!("{phase}.bmp")))
            .unwrap();
        assert!(
            Command::new("/usr/sbin/screencapture")
                .args([
                    "-x",
                    "-R",
                    &format!("{},{},{},{}", rect.x, rect.y, rect.width, rect.height)
                ])
                .arg(self.artifacts.join(format!("{phase}.png")))
                .status()
                .unwrap()
                .success()
        );
        println!("AppKit checkpoint: {phase}");
    }
}
impl Drop for NativeTarget {
    fn drop(&mut self) {
        for key in [
            "shift",
            "right_shift",
            "control",
            "right_control",
            "alt",
            "right_alt",
            "command",
        ] {
            let _ = Robot.key_toggle(key, "up", &[]);
        }
        if thread::panicking() && !self.geometry.is_null() {
            let _ =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.evidence("failure")));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = Robot.move_mouse(self.cursor.x, self.cursor.y);
        let _ = Robot.set_mouse_delay(10);
        let _ = Robot.set_keyboard_delay(10);
    }
}

#[test]
#[ignore = "requires an idle macOS desktop, permissions, and Xcode command line tools"]
fn native_appkit_typing_editing_and_independent_modifier_keys() {
    let mut target = NativeTarget::start();
    target.focus("button");
    assert_eq!(target.state(json!({}))["clicks"], 1);
    target.focus("editor");
    Robot
        .type_string("Native AppKit: café e\u{301} 中文 👩‍💻 🚀\nSecond line")
        .unwrap();
    Robot.unicode_tap('\n').unwrap();
    Robot.type_string_delayed("Paced\ntext", 6000).unwrap();
    target.text("Native AppKit: café e\u{301} 中文 👩‍💻 🚀\nSecond line\nPaced\ntext");
    target.evidence("01-native-unicode");
    Robot.key_tap("a", &["command"]).unwrap();
    assert!(target.state(json!({}))["selectionLength"].as_u64().unwrap() > 0);
    Robot.type_string("replace").unwrap();
    Robot.key_tap("left", &["shift"]).unwrap();
    Robot.type_string("E").unwrap();
    target.text("replacE");
    Robot.key_tap("z", &["command"]).unwrap();
    target.text("replace");
    Robot.key_tap("z", &["command", "shift"]).unwrap();
    target.text("replacE");
    target.evidence("02-native-editing");
    Robot.key_tap("a", &["command"]).unwrap();
    Robot.key_tap("backspace", &[]).unwrap();
    for (first, second) in [("shift", "right_shift"), ("right_shift", "shift")] {
        Robot.key_toggle(first, "down", &[]).unwrap();
        Robot.key_toggle(second, "down", &[]).unwrap();
        Robot.key_toggle(first, "up", &[]).unwrap();
        Robot.key_tap("a", &[]).unwrap();
        Robot.key_toggle(second, "up", &[]).unwrap();
        Robot.key_tap("a", &[]).unwrap();
    }
    target.text("AaAa");
    target.evidence("03-independent-shift-keys");
}
