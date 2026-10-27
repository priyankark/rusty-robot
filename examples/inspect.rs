//! Read-only desktop diagnostics; optionally save the main screen to a file.
use rusty_robot::Robot;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Accessibility: {:?}", Robot.get_accessibility_permission()?);
    println!(
        "Screen capture: {:?}",
        Robot.get_screen_capture_permission()?
    );
    println!("Displays: {:?}", Robot.get_displays()?);
    println!("Pointer: {:?}", Robot.get_mouse_pos()?);
    if let Some(path) = std::env::args_os().nth(1) {
        Robot.capture(None)?.save(std::path::Path::new(&path))?;
    }
    Ok(())
}
