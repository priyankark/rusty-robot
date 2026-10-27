use crate::{
    Point, Result, Robot,
    error::err,
    platform::{self, Button},
};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};
static DELAY: AtomicU64 = AtomicU64::new(10);
fn wait() {
    thread::sleep(Duration::from_millis(DELAY.load(Ordering::Relaxed)));
}
impl Robot {
    pub fn set_mouse_delay(&self, ms: u64) -> Result<()> {
        DELAY.store(ms, Ordering::Relaxed);
        Ok(())
    }
    pub fn get_mouse_pos(&self) -> Result<Point> {
        platform::mouse_position()
    }
    pub fn move_mouse(&self, x: i64, y: i64) -> Result<()> {
        let (x, y) = (platform::coordinate(x)?, platform::coordinate(y)?);
        let _lock = platform::input_lock()?;
        platform::move_to(x, y, None)?;
        wait();
        Ok(())
    }
    pub fn move_mouse_smooth(&self, x: i64, y: i64, speed: Option<f64>) -> Result<()> {
        let (x, y) = (platform::coordinate(x)?, platform::coordinate(y)?);
        let speed = speed.unwrap_or(3.0);
        if !speed.is_finite() || speed <= 0.0 {
            return Err(err("Mouse speed must be positive and finite"));
        }
        let _lock = platform::input_lock()?;
        let start = platform::mouse_position()?;
        let distance = (x as f64 - start.x as f64).hypot(y as f64 - start.y as f64);
        let duration = Duration::from_secs_f64((distance * speed / 2000.0).clamp(0.001, 1.0));
        let steps = (duration.as_secs_f64() * 120.0).ceil() as u32;
        let begun = Instant::now();
        for step in 1..=steps {
            let fraction = step as f64 / steps as f64;
            let t = fraction * fraction * (3.0 - 2.0 * fraction);
            let px = (start.x as f64 + (x as f64 - start.x as f64) * t).round() as i32;
            let py = (start.y as f64 + (y as f64 - start.y as f64) * t).round() as i32;
            platform::move_to(px, py, None)?;
            if let Some(remaining) =
                (begun + duration.mul_f64(fraction)).checked_duration_since(Instant::now())
            {
                thread::sleep(remaining);
            }
        }
        wait();
        Ok(())
    }
    pub fn mouse_click(&self, button: Option<&str>, double: bool) -> Result<()> {
        let button = Button::parse(button)?;
        let _lock = platform::input_lock()?;
        for count in 1..=if double { 2 } else { 1 } {
            platform::button(button, true, count)?;
            platform::button(button, false, count)?;
            if double && count == 1 {
                thread::sleep(Duration::from_millis(50));
            }
        }
        wait();
        Ok(())
    }
    pub fn drag_mouse(&self, x: i64, y: i64, button: Option<&str>) -> Result<()> {
        let button = Button::parse(button)?;
        let (x, y) = (platform::coordinate(x)?, platform::coordinate(y)?);
        let _lock = platform::input_lock()?;
        platform::move_to(x, y, Some(button))?;
        wait();
        Ok(())
    }
    pub fn scroll_mouse(&self, x: i64, y: i64) -> Result<()> {
        let (x, y) = (platform::coordinate(x)?, platform::coordinate(y)?);
        let _lock = platform::input_lock()?;
        platform::scroll(x, y)?;
        wait();
        Ok(())
    }
    pub fn mouse_toggle(&self, state: &str, button: Option<&str>) -> Result<()> {
        let down = platform::down(state)?;
        let button = Button::parse(button)?;
        let _lock = platform::input_lock()?;
        platform::button(button, down, 1)?;
        wait();
        Ok(())
    }
}
