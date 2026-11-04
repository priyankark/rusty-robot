use crate::{Rect, Result, Robot, error::err, platform};
/// Native window identity and observed logical desktop bounds. Identity is
/// validated against the owning PID on every operation; OS IDs may be recycled.
#[derive(Debug, Clone)]
pub struct TargetWindow {
    pub id: u64,
    pub process_id: u32,
    pub bounds: Rect,
    pub title: String,
    pub active: bool,
}
impl Robot {
    /// Enumerate visible top-level windows without activating any of them.
    pub fn windows(&self) -> Result<Vec<TargetWindow>> {
        platform::windows()
    }
}
impl TargetWindow {
    pub fn refresh(&self) -> Result<Self> {
        platform::windows()?
            .into_iter()
            .find(|w| w.id == self.id && w.process_id == self.process_id)
            .ok_or_else(|| err("Target window disappeared or changed owner"))
    }
    /// Explicit request; foreground restrictions can deny it. Re-observe before input.
    /// On macOS this activates the owner application; verify the desired window
    /// with refresh/require_active afterwards when the app has multiple windows.
    pub fn activate(&self) -> Result<()> {
        let current = self.refresh()?;
        platform::activate_window(&current)
    }
    /// Checks the foreground window now, not the focused control or unlocked state.
    /// Focus can still change between this check and an input call.
    pub fn require_active(&self) -> Result<()> {
        if self.refresh()?.active {
            Ok(())
        } else {
            Err(err("Target window is not active"))
        }
    }
}
