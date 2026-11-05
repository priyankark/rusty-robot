//! Opt-in sequence ownership and scoped input release.
//! Sessions exclude other sessions in this process, not legacy Robot calls or
//! other processes. All cooperating input producers must use the session API.
use crate::{Point, Result, Robot, error::err};
use std::sync::{Mutex, MutexGuard, TryLockError};
static SESSION: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}
impl MouseButton {
    fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Middle => "middle",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeldInput {
    Button(MouseButton),
    Key(String),
}
/// Injection boundary for owned input, also useful for adapter fault testing.
pub trait InputBackend {
    fn toggle(&mut self, input: &HeldInput, down: bool) -> Result<()>;
    fn move_pointer(&mut self, point: Point, drag: Option<MouseButton>) -> Result<()>;
}
impl InputBackend for Robot {
    fn toggle(&mut self, input: &HeldInput, down: bool) -> Result<()> {
        let state = if down { "down" } else { "up" };
        match input {
            HeldInput::Button(b) => self.mouse_toggle(state, Some(b.name())),
            HeldInput::Key(k) => self.key_toggle(k, state, &[]),
        }
    }
    fn move_pointer(&mut self, p: Point, drag: Option<MouseButton>) -> Result<()> {
        match drag {
            Some(b) => self.drag_mouse(p.x, p.y, Some(b.name())),
            None => self.move_mouse(p.x, p.y),
        }
    }
}
pub struct InputSession<B: InputBackend> {
    backend: B,
    _ownership: MutexGuard<'static, ()>,
}
impl Robot {
    pub fn input_session(&self) -> Result<InputSession<Robot>> {
        InputSession::new(Robot)
    }
}
impl<B: InputBackend> InputSession<B> {
    /// Fails rather than deadlocking on nested/concurrent sessions.
    pub fn new(backend: B) -> Result<Self> {
        Ok(Self {
            backend,
            // The mutex protects ownership, not mutable state. Guards release
            // held input during unwinding, so a prior panic need not disable it.
            _ownership: match SESSION.try_lock() {
                Ok(owner) => owner,
                Err(TryLockError::Poisoned(poison)) => poison.into_inner(),
                Err(TryLockError::WouldBlock) => {
                    return Err(err("An input session is already active"));
                }
            },
        })
    }
    pub fn hold(&mut self, input: HeldInput) -> Result<InputGuard<'_, B>> {
        // Create the guard before pressing: partial errors and panic unwinding
        // must release even if the backend posted the press before failing.
        let guard = InputGuard {
            backend: &mut self.backend,
            input,
            held: true,
        };
        if let Err(press) = guard.backend.toggle(&guard.input, true) {
            return match guard.finish() {
                Ok(()) => Err(press),
                Err(release) => Err(err(format!("{press}; release also failed: {release}"))),
            };
        }
        Ok(guard)
    }
    /// Caller supplies a path and cooperative cancellation. Release runs on
    /// error/cancellation and panic unwind; forced process termination is excluded.
    pub fn drag_path(
        &mut self,
        button: MouseButton,
        start: Point,
        path: impl IntoIterator<Item = Point>,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<()> {
        if cancelled() {
            return Err(err("Drag cancelled"));
        }
        self.backend.move_pointer(start, None)?;
        let mut guard = self.hold(HeldInput::Button(button))?;
        let movement = (|| {
            for p in path {
                if cancelled() {
                    return Err(err("Drag cancelled"));
                }
                guard.move_pointer(p)?;
            }
            if cancelled() {
                return Err(err("Drag cancelled"));
            }
            Ok(())
        })();
        let release = guard.finish();
        match (movement, release) {
            (Err(a), Err(b)) => Err(err(format!("{a}; release also failed: {b}"))),
            (Err(e), _) | (_, Err(e)) => Err(e),
            _ => Ok(()),
        }
    }
}
pub struct InputGuard<'a, B: InputBackend> {
    backend: &'a mut B,
    input: HeldInput,
    held: bool,
}
impl<B: InputBackend> InputGuard<'_, B> {
    pub fn move_pointer(&mut self, p: Point) -> Result<()> {
        let button = match self.input {
            HeldInput::Button(b) => Some(b),
            HeldInput::Key(_) => None,
        };
        self.backend.move_pointer(p, button)
    }
    /// Explicit release reports errors. On failure Drop retries once, best effort.
    pub fn finish(mut self) -> Result<()> {
        self.backend.toggle(&self.input, false)?;
        self.held = false;
        Ok(())
    }
}
impl<B: InputBackend> Drop for InputGuard<'_, B> {
    fn drop(&mut self) {
        if self.held {
            let _ = self.backend.toggle(&self.input, false);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    struct Fake {
        events: Rc<RefCell<Vec<&'static str>>>,
        fail: usize,
        moves: usize,
        release_fail: bool,
        press_fail: bool,
    }
    impl InputBackend for Fake {
        fn toggle(&mut self, _: &HeldInput, down: bool) -> Result<()> {
            self.events
                .borrow_mut()
                .push(if down { "down" } else { "up" });
            if down && self.press_fail {
                Err(err("partial press"))
            } else if !down && self.release_fail {
                self.release_fail = false;
                Err(err("release"))
            } else {
                Ok(())
            }
        }
        fn move_pointer(&mut self, _: Point, drag: Option<MouseButton>) -> Result<()> {
            if drag.is_some() {
                self.moves += 1;
                if self.moves == self.fail {
                    return Err(err("movement"));
                }
            }
            self.events.borrow_mut().push("move");
            Ok(())
        }
    }
    #[test]
    fn failures_cancellation_nesting_and_drop_release_owned_input() {
        // One test owns the process-wide session mutex; no parallel desktop effects.
        for fail in 1..=4 {
            let events = Rc::new(RefCell::new(vec![]));
            let fake = Fake {
                events: events.clone(),
                fail,
                moves: 0,
                release_fail: false,
                press_fail: false,
            };
            let mut s = InputSession::new(fake).unwrap();
            assert!(Robot.input_session().is_err());
            assert!(
                s.drag_path(
                    MouseButton::Left,
                    Point::default(),
                    [Point::default(); 4],
                    || false
                )
                .is_err()
            );
            assert_eq!(events.borrow().iter().filter(|e| **e == "up").count(), 1);
        }
        let events = Rc::new(RefCell::new(vec![]));
        let mut s = InputSession::new(Fake {
            events: events.clone(),
            fail: 0,
            moves: 0,
            release_fail: true,
            press_fail: false,
        })
        .unwrap();
        let mut checks = 0;
        let e = s
            .drag_path(
                MouseButton::Left,
                Point::default(),
                [Point::default(); 4],
                || {
                    checks += 1;
                    checks == 3
                },
            )
            .unwrap_err();
        assert!(e.to_string().contains("cancelled"));
        assert!(e.to_string().contains("release"));
        assert_eq!(events.borrow().iter().filter(|e| **e == "up").count(), 2);
        {
            let _guard = s.hold(HeldInput::Key("shift".into())).unwrap();
        }
        assert_eq!(*events.borrow().last().unwrap(), "up");
        s.backend.press_fail = true;
        assert!(s.hold(HeldInput::Button(MouseButton::Left)).is_err());
        assert_eq!(*events.borrow().last().unwrap(), "up");
        s.backend.press_fail = false;
        // Catch unwind before dropping the session so the ownership mutex stays usable.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = s.hold(HeldInput::Button(MouseButton::Left)).unwrap();
            panic!("abort operation");
        }));
        assert_eq!(*events.borrow().last().unwrap(), "up");
        drop(s);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut s = InputSession::new(Fake {
                events: events.clone(),
                fail: 0,
                moves: 0,
                release_fail: false,
                press_fail: false,
            })
            .unwrap();
            let _guard = s.hold(HeldInput::Button(MouseButton::Left)).unwrap();
            panic!("unwind whole session");
        }));
        assert_eq!(*events.borrow().last().unwrap(), "up");
        assert!(Robot.input_session().is_ok());
    }
}
