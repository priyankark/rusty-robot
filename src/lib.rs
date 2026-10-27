//! Cross-platform desktop automation in Rust.
#![doc = include_str!("../skills/rusty-robot/references/rust-api.md")]
#![doc = include_str!("../docs/type-contracts.md")]
mod bitmap;
mod error;
pub mod image;
mod types;
pub use error::{Error, Result};
pub use types::*;
mod keyboard;
mod mouse;
mod permissions;
mod platform;
mod screen;

/// Stateless handle to the desktop; delay settings are process-wide.
#[derive(Debug, Default)]
pub struct Robot;
impl MouseOutput for Robot {
    fn move_mouse(&mut self, x: i64, y: i64) -> Result<()> {
        Robot::move_mouse(self, x, y)
    }
    fn mouse_click(&mut self, button: Option<&str>, double: bool) -> Result<()> {
        Robot::mouse_click(self, button, double)
    }
}
