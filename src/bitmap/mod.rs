//! Validated BGR/BGRA storage, bounded search, and capture coordinate conversion.
mod search;
use crate::{Bitmap, MouseOutput, Point, Result, SearchOptions, Size, error::err};
use std::path::Path;

impl Bitmap {
    /// Validate metadata before performing any indexing or allocation.
    pub fn validate(&self) -> Result<()> {
        for (name, value) in [
            ("width", self.width),
            ("height", self.height),
            ("byteWidth", self.byte_width),
        ] {
            if value > isize::MAX as usize {
                return Err(err(format!("{name} is outside the supported range")));
            }
        }
        if self.width == 0 || self.height == 0 {
            return Err(err("Bitmap dimensions must be positive"));
        }
        if self.bits_per_pixel > u8::MAX as u16 || self.bytes_per_pixel > u8::MAX as u16 {
            return Err(err("Bitmap pixel metadata is outside the supported range"));
        }
        if !matches!(
            (self.bits_per_pixel, self.bytes_per_pixel),
            (24, 3) | (32, 4)
        ) {
            return Err(err("Bitmap must use 24-bit or 32-bit pixels"));
        }
        let row = self
            .width
            .checked_mul(self.bytes_per_pixel as usize)
            .ok_or_else(|| err("Bitmap row size is outside the supported range"))?;
        if self.byte_width < row {
            return Err(err("byteWidth is smaller than the bitmap row size"));
        }
        let size = self
            .byte_width
            .checked_mul(self.height)
            .filter(|n| *n <= isize::MAX as usize)
            .ok_or_else(|| err("Bitmap buffer size is outside the supported range"))?;
        if self.image.len() < size {
            return Err(err("image buffer is smaller than byteWidth * height"));
        }
        Ok(())
    }
    /// Construct an owned, tightly packed BGRA bitmap without unchecked allocation.
    pub(crate) fn empty(width: usize, height: usize) -> Result<Self> {
        let stride = width
            .checked_mul(4)
            .ok_or_else(|| err("Bitmap row size is outside the supported range"))?;
        let len = stride
            .checked_mul(height)
            .filter(|v| *v <= isize::MAX as usize)
            .ok_or_else(|| err("Bitmap buffer size is outside the supported range"))?;
        let mut image = Vec::new();
        image
            .try_reserve_exact(len)
            .map_err(|e| err(e.to_string()))?;
        image.resize(len, 0);
        let bitmap = Self {
            width,
            height,
            byte_width: stride,
            bits_per_pixel: 32,
            bytes_per_pixel: 4,
            image,
            screen_x: 0,
            screen_y: 0,
            scale_x: 1.0,
            scale_y: 1.0,
        };
        bitmap.validate()?;
        Ok(bitmap)
    }
    pub(crate) fn rgb(&self, x: usize, y: usize) -> [u8; 3] {
        let i = y * self.byte_width + x * self.bytes_per_pixel as usize;
        [self.image[i + 2], self.image[i + 1], self.image[i]]
    }
    pub fn color_at(&self, x: i64, y: i64) -> Result<String> {
        self.validate()?;
        if x < 0 || y < 0 || x as u64 >= self.width as u64 || y as u64 >= self.height as u64 {
            return Err(err("Coordinates are outside the bitmap"));
        }
        let [r, g, b] = self.rgb(x as usize, y as usize);
        Ok(format!("{r:02x}{g:02x}{b:02x}"))
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        crate::image::save(self, path)
    }
    pub fn to_screen_point(&self, point: Point, target: Option<Size>) -> Result<Point> {
        let size = target.unwrap_or_default();
        fn axis(origin: i64, point: i64, size: usize, scale: f64) -> Result<i64> {
            if !scale.is_finite() {
                return Err(err("Capture scale must be finite"));
            }
            let scale = if scale > 0.0 { scale } else { 1.0 };
            let value = origin as f64 + (point as f64 + (size / 2) as f64) / scale;
            // Match Math.round, including negative half-integers.
            let rounded = (value + 0.5).floor();
            if !rounded.is_finite() || rounded < i64::MIN as f64 || rounded >= -(i64::MIN as f64) {
                return Err(err("Screen coordinate is outside the supported range"));
            }
            Ok(rounded as i64)
        }
        Ok(Point {
            x: axis(self.screen_x, point.x, size.width, self.scale_x)?,
            y: axis(self.screen_y, point.y, size.height, self.scale_y)?,
        })
    }
    pub fn click(
        &self,
        output: &mut dyn MouseOutput,
        point: Point,
        target: Option<Size>,
        button: Option<&str>,
        double: bool,
    ) -> Result<Point> {
        let point = self.to_screen_point(point, target)?;
        let button = button.unwrap_or("left");
        if !matches!(button, "left" | "right" | "middle") {
            return Err(err("Invalid mouse button specified."));
        }
        output.move_mouse(point.x, point.y)?;
        output.mouse_click(Some(button), double)?;
        Ok(point)
    }
    pub fn click_image(
        &self,
        output: &mut dyn MouseOutput,
        needle: &Bitmap,
        options: SearchOptions,
        button: Option<&str>,
        double: bool,
    ) -> Result<Option<Point>> {
        if let Some(point) = self.find_image(needle, options)? {
            self.click(
                output,
                point,
                Some(Size {
                    width: needle.width,
                    height: needle.height,
                }),
                button,
                double,
            )?;
            Ok(Some(point))
        } else {
            Ok(None)
        }
    }
}
