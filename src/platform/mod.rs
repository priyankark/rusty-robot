//! Platform boundary. All native resources stay inside a backend.
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub(crate) use macos::*;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub(crate) use linux::*;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub(crate) use windows::*;
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
compile_error!("rusty-robot supports macOS, Linux/X11 and Windows");
use crate::{Result, error::err};
use std::sync::{Mutex, MutexGuard};
static INPUT: Mutex<()> = Mutex::new(());
pub(crate) fn input_lock() -> Result<MutexGuard<'static, ()>> {
    INPUT
        .lock()
        .map_err(|_| err("Desktop input lock was poisoned"))
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum Button {
    Left,
    Right,
    Middle,
}
impl Button {
    pub(crate) fn parse(button: Option<&str>) -> Result<Self> {
        match button.unwrap_or("left") {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            "middle" => Ok(Self::Middle),
            _ => Err(err("Invalid mouse button specified.")),
        }
    }
}
pub(crate) fn coordinate(value: i64) -> Result<i32> {
    i32::try_from(value).map_err(|_| err("Coordinate is outside the supported range"))
}
pub(crate) fn down(state: &str) -> Result<bool> {
    match state {
        "down" => Ok(true),
        "up" => Ok(false),
        _ => Err(err("Invalid key or mouse button state specified.")),
    }
}
