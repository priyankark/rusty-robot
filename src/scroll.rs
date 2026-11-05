use crate::{Result, Robot, error::err, platform};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollUnit {
    Pixels,
    WheelSteps,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollCapabilities {
    pub pixels: bool,
    pub wheel_steps: bool,
}
impl Robot {
    /// Exact event units, not a guarantee of application displacement.
    pub fn scroll_capabilities(&self) -> ScrollCapabilities {
        ScrollCapabilities {
            pixels: cfg!(target_os = "macos"),
            wheel_steps: true,
        }
    }
    /// Positive x scrolls left, positive y scrolls up, matching scroll_mouse.
    /// Pixel units are currently supported only on macOS. WheelSteps are whole
    /// detents (line events on macOS); each axis is limited to 1,000 per call.
    /// The original backend-native scroll_mouse method is unchanged.
    pub fn scroll(&self, x: i64, y: i64, unit: ScrollUnit) -> Result<()> {
        let (x, y) = validate(x, y, unit, self.scroll_capabilities())?;
        let _lock = platform::input_lock()?;
        match unit {
            ScrollUnit::Pixels => platform::scroll(x, y),
            ScrollUnit::WheelSteps => platform::scroll_steps(x, y),
        }
    }
}
fn validate(x: i64, y: i64, unit: ScrollUnit, c: ScrollCapabilities) -> Result<(i32, i32)> {
    if !(match unit {
        ScrollUnit::Pixels => c.pixels,
        ScrollUnit::WheelSteps => c.wheel_steps,
    }) {
        return Err(err("Requested scroll unit is unsupported on this backend"));
    }
    let (x, y) = (platform::coordinate(x)?, platform::coordinate(y)?);
    if unit == ScrollUnit::WheelSteps && (x.unsigned_abs() > 1000 || y.unsigned_abs() > 1000) {
        return Err(err("Wheel scrolling is limited to 1000 steps per axis"));
    }
    Ok((x, y))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_and_excessive_scrolls_fail_before_native_input() {
        let c = ScrollCapabilities {
            pixels: false,
            wheel_steps: true,
        };
        assert!(validate(0, 600, ScrollUnit::Pixels, c).is_err());
        for n in [1001, i32::MIN as i64, i64::MAX] {
            assert!(validate(n, 0, ScrollUnit::WheelSteps, c).is_err());
        }
        assert_eq!(validate(-3, 2, ScrollUnit::WheelSteps, c).unwrap(), (-3, 2));
    }
}
