use crate::{Result, Robot, platform};
impl Robot {
    pub fn get_accessibility_permission(&self) -> Result<Option<bool>> {
        platform::accessibility(false)
    }
    pub fn request_accessibility_permission(&self) -> Result<Option<bool>> {
        platform::accessibility(true)
    }
    pub fn get_screen_capture_permission(&self) -> Result<Option<bool>> {
        platform::screen_permission(false)
    }
    pub fn request_screen_capture_permission(&self) -> Result<Option<bool>> {
        platform::screen_permission(true)
    }
}
