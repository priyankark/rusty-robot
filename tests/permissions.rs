use rusty_robot::{Result, Robot};
#[test]
fn reports_permission_status_without_prompting() {
    let accessibility = Robot.get_accessibility_permission().unwrap();
    let screen_capture = Robot.get_screen_capture_permission().unwrap();
    if cfg!(target_os = "macos") {
        assert!(accessibility.is_some());
        assert!(screen_capture.is_some());
    } else {
        assert_eq!(accessibility, None);
        assert_eq!(screen_capture, None);
    }
}
#[test]
fn exposes_explicit_permission_requests() {
    let accessibility: fn(&Robot) -> Result<Option<bool>> = Robot::request_accessibility_permission;
    let screen_capture: fn(&Robot) -> Result<Option<bool>> =
        Robot::request_screen_capture_permission;
    // Upstream never triggers a permission dialog during a test on macOS.
    if !cfg!(target_os = "macos") {
        assert_eq!(accessibility(&Robot).unwrap(), None);
        assert_eq!(screen_capture(&Robot).unwrap(), None);
    }
}
