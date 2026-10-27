use crate::{Bitmap, Display, Rect, Result, Robot, Size, error::err, platform};
impl Robot {
    pub fn get_displays(&self) -> Result<Vec<Display>> {
        platform::displays()
    }
    pub fn get_screen_size(&self) -> Result<Size> {
        let display = self
            .get_displays()?
            .into_iter()
            .find(|d| d.is_main)
            .ok_or_else(|| err("No main display is available"))?;
        Ok(Size {
            width: display.width,
            height: display.height,
        })
    }
    pub fn capture(&self, rect: Option<Rect>) -> Result<Bitmap> {
        let displays = self.get_displays()?;
        let rect = match rect {
            Some(rect) => rect,
            None => {
                let d = displays
                    .iter()
                    .find(|d| d.is_main)
                    .ok_or_else(|| err("No main display is available"))?;
                Rect {
                    x: d.x,
                    y: d.y,
                    width: d.width,
                    height: d.height,
                }
            }
        };
        if rect.width == 0 || rect.height == 0 {
            return Err(err("Capture dimensions must be positive"));
        }
        let end_x = rect.x as i128 + rect.width as i128;
        let end_y = rect.y as i128 + rect.height as i128;
        let display = displays
            .iter()
            .find(|d| {
                rect.x >= d.x
                    && rect.y >= d.y
                    && end_x <= d.x as i128 + d.width as i128
                    && end_y <= d.y as i128 + d.height as i128
            })
            .ok_or_else(|| err("Capture rectangle is outside a single display"))?;
        let mut bitmap = platform::capture(display, rect)?;
        bitmap.validate()?;
        bitmap.screen_x = rect.x;
        bitmap.screen_y = rect.y;
        bitmap.scale_x = bitmap.width as f64 / rect.width as f64;
        bitmap.scale_y = bitmap.height as f64 / rect.height as f64;
        Ok(bitmap)
    }
    pub fn get_pixel_color(&self, x: i64, y: i64) -> Result<String> {
        if x < 0 || y < 0 {
            return Err(err("Pixel coordinates must be non-negative"));
        }
        let size = self.get_screen_size()?;
        if x as u64 >= size.width as u64 || y as u64 >= size.height as u64 {
            return Err(err("Coordinates are outside the main screen"));
        }
        self.capture(Some(Rect {
            x,
            y,
            width: 1,
            height: 1,
        }))?
        .color_at(0, 0)
    }
}
