//! Rust port of test/helpers/targetpractice.js. Electron is only the target UI.
use rusty_robot::{Point, Robot};
use serde_json::Value;
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

pub const TARGET_COLOR: &str = "c0ff33";
const TIMEOUT: Duration = Duration::from_secs(5);
pub struct Target {
    child: Child,
    events: Receiver<Value>,
    elements: Value,
}
impl Drop for Target {
    fn drop(&mut self) {
        // Let Electron stop its renderer children before the next fixture starts.
        #[cfg(unix)]
        unsafe {
            libc::kill(self.child.id() as i32, libc::SIGTERM);
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            if Instant::now() >= deadline {
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Target {
    pub fn start(interactive: bool) -> Self {
        // Resolve the pinned fixture only. The RobotJS addon is never loaded.
        let resolved=Command::new("node").args(["-e", "const p=require('path'); process.stdout.write(JSON.stringify({electron:require('electron'),app:p.join(p.dirname(require.resolve('targetpractice')),'electron-app')}))"])
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"),"/tests/gui")).output().expect("Node.js required for Target Practice");
        assert!(
            resolved.status.success(),
            "Install fixture with npm ci --prefix tests/gui: {}",
            String::from_utf8_lossy(&resolved.stderr)
        );
        let paths: Value = serde_json::from_slice(&resolved.stdout).unwrap();
        let mut command = Command::new(paths["electron"].as_str().unwrap());
        if cfg!(target_os = "linux") {
            command.args(["--no-sandbox", "--disable-setuid-sandbox"]);
        }
        let mut child = command
            .arg(paths["app"].as_str().unwrap())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("launch Target Practice");
        let stdout = child.stdout.take().unwrap();
        let (sender, events) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if let Ok(event) = serde_json::from_str::<Value>(&line)
                    && sender.send(event).is_err()
                {
                    break;
                }
            }
        });
        // Own the child before any assertions, so failed readiness also cleans up.
        let mut target = Self {
            child,
            events,
            elements: Value::Null,
        };
        target.elements = target.wait_until("elements", Duration::from_secs(15), |_| true);
        target.bring_to_front();
        let marker = target.point("color_1");
        let deadline = Instant::now() + TIMEOUT;
        let mut stable = 0;
        let mut focused_at = Instant::now();
        loop {
            let capture = Robot.capture(None).expect("sample Target Practice marker");
            let mut colors = Vec::new();
            for (dx, dy) in [(0, 0), (-25, -25), (24, -25), (-25, 24), (24, 24)] {
                let x =
                    ((marker.x + dx - capture.screen_x) as f64 * capture.scale_x).round() as i64;
                let y =
                    ((marker.y + dy - capture.screen_y) as f64 * capture.scale_y).round() as i64;
                colors.push(capture.color_at(x, y).expect("sample marker bounds"));
            }
            if colors.iter().all(|c| c == TARGET_COLOR) {
                stable += 1;
            } else {
                stable = 0;
            }
            if stable == 3 {
                break;
            }
            if stable == 0 && focused_at.elapsed() >= Duration::from_secs(1) {
                target.bring_to_front();
                focused_at = Instant::now();
            }
            assert!(
                Instant::now() < deadline,
                "Target Practice marker did not settle at its reported bounds; sampled colors: {colors:?}"
            );
            thread::sleep(Duration::from_millis(25));
        }
        if interactive {
            let button = target.point("button_1");
            let deadline = Instant::now() + TIMEOUT;
            loop {
                Robot.move_mouse(button.x, button.y).unwrap();
                Robot.mouse_click(None, false).unwrap();
                if target
                    .receive_matching("click", Duration::from_millis(100), |e| {
                        e["id"] == "button_1"
                    })
                    .is_some()
                {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "Target Practice did not accept mouse input"
                );
            }
        }
        target.drain();
        target
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
            // SAFETY: Activate only the disposable Electron process that this
            // fixture owns. macOS Space transitions can otherwise leave it behind
            // the runner despite Electron having emitted its focus/readiness event.
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
    pub fn point(&self, id: &str) -> Point {
        let element = &self.elements[id];
        let coord = |name| {
            element[name]
                .as_f64()
                .unwrap_or_else(|| panic!("missing {id}.{name}"))
                .round() as i64
        };
        Point {
            x: coord("x"),
            y: coord("y"),
        }
    }
    pub fn drain(&self) {
        while self.events.try_recv().is_ok() {}
    }
    fn receive_matching(
        &self,
        event: &str,
        timeout: Duration,
        predicate: impl Fn(&Value) -> bool,
    ) -> Option<Value> {
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.checked_duration_since(Instant::now())?;
            match self.events.recv_timeout(remaining) {
                Ok(value) => {
                    assert_ne!(value["event"], "error", "Target Practice error: {value}");
                    if value["event"] == event && predicate(&value["message"]) {
                        return Some(value["message"].clone());
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => return None,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    panic!("Target Practice exited before {event}")
                }
            }
        }
    }
    fn wait_until(
        &self,
        event: &str,
        timeout: Duration,
        predicate: impl Fn(&Value) -> bool,
    ) -> Value {
        self.receive_matching(event, timeout, predicate)
            .unwrap_or_else(|| {
                panic!("Timed out after {timeout:?} waiting for Target Practice {event}")
            })
    }
    pub fn event(&self, event: &str) -> Value {
        self.wait_until(event, TIMEOUT, |_| true)
    }
    pub fn typed(&self, expected: &str) {
        self.wait_until("type", TIMEOUT, |e| {
            e["id"] == "input_1" && e["text"] == expected
        });
    }
    pub fn focus(&self, id: &str) {
        let p = self.point(id);
        Robot.move_mouse(p.x, p.y).unwrap();
        Robot.mouse_click(None, false).unwrap();
    }
}
