use crate::{Bitmap, Point, Result, SearchOptions, error::err};
use std::ops::Range;
fn color(hex: &str) -> Result<[u8; 3]> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(err("Color must be a 6-digit hex string"));
    }
    let value =
        u32::from_str_radix(hex, 16).map_err(|_| err("Color must be a 6-digit hex string"))?;
    Ok([(value >> 16) as u8, (value >> 8) as u8, value as u8])
}
fn similar(a: [u8; 3], b: [u8; 3], threshold: f64) -> bool {
    // RobotJS avoids Euclidean-distance arithmetic for exact matches.
    if threshold == 0.0 {
        return a == b;
    }
    let distance: u32 = a
        .iter()
        .zip(b)
        .map(|(a, b)| {
            let d = i32::from(*a) - i32::from(b);
            (d * d) as u32
        })
        .sum();
    distance as f64 <= threshold
}
impl Bitmap {
    fn bounds(&self, options: SearchOptions) -> Result<(Range<usize>, Range<usize>, f64)> {
        self.validate()?;
        for (field, value) in [
            ("x", options.x),
            ("y", options.y),
            ("width", options.width),
            ("height", options.height),
        ] {
            if value.is_some_and(|v| v > isize::MAX as usize) {
                return Err(err(format!("{field} is outside the supported range")));
            }
        }
        if !options.tolerance.is_finite() || !(0.0..=1.0).contains(&options.tolerance) {
            return Err(err("tolerance must be between 0.0 and 1.0"));
        }
        let x = options.x.unwrap_or(0);
        let y = options.y.unwrap_or(0);
        let width = options.width.unwrap_or(self.width.saturating_sub(x));
        let height = options.height.unwrap_or(self.height.saturating_sub(y));
        let end_x = x.checked_add(width).filter(|v| *v <= self.width);
        let end_y = y.checked_add(height).filter(|v| *v <= self.height);
        if x > self.width || y > self.height || end_x.is_none() || end_y.is_none() {
            return Err(err("Search rectangle is outside the bitmap"));
        }
        Ok((
            x..end_x.unwrap(),
            y..end_y.unwrap(),
            (options.tolerance * 442.0).powi(2),
        ))
    }
    fn scan_color(
        &self,
        hex: &str,
        options: SearchOptions,
        mut found: impl FnMut(Point) -> bool,
    ) -> Result<()> {
        let rgb = color(hex)?;
        let (xs, ys, threshold) = self.bounds(options)?;
        if threshold == 0.0 {
            let bgr = [rgb[2], rgb[1], rgb[0]];
            let packed = u32::from_le_bytes([bgr[0], bgr[1], bgr[2], 0]);
            let bpp = self.bytes_per_pixel as usize;
            for y in ys {
                let start = y * self.byte_width;
                let row = &self.image[start + xs.start * bpp..start + xs.end * bpp];
                if bpp == 4 {
                    for (offset, pixel) in row.as_chunks::<4>().0.iter().enumerate() {
                        // RobotJS compares packed RGB values, ignoring alpha.
                        if u32::from_le_bytes(*pixel) & 0x00ff_ffff == packed
                            && !found(Point {
                                x: (xs.start + offset) as i64,
                                y: y as i64,
                            })
                        {
                            return Ok(());
                        }
                    }
                } else {
                    for (offset, pixel) in row.as_chunks::<3>().0.iter().enumerate() {
                        if *pixel == bgr
                            && !found(Point {
                                x: (xs.start + offset) as i64,
                                y: y as i64,
                            })
                        {
                            return Ok(());
                        }
                    }
                }
            }
            return Ok(());
        }
        for y in ys {
            for x in xs.clone() {
                if similar(self.rgb(x, y), rgb, threshold)
                    && !found(Point {
                        x: x as i64,
                        y: y as i64,
                    })
                {
                    return Ok(());
                }
            }
        }
        Ok(())
    }
    fn scan_image(
        &self,
        needle: &Bitmap,
        options: SearchOptions,
        mut found: impl FnMut(Point) -> bool,
    ) -> Result<()> {
        let (xs, ys, threshold) = self.bounds(options)?;
        needle.validate()?;
        if needle.width > xs.len() || needle.height > ys.len() {
            return Ok(());
        }
        for y in ys.start..=ys.end - needle.height {
            'candidate: for x in xs.start..=xs.end - needle.width {
                // Match RobotJS's backwards needle comparison; candidate positions
                // still advance in row-major order, including overlapping matches.
                for ny in (0..needle.height).rev() {
                    for nx in (0..needle.width).rev() {
                        if !similar(self.rgb(x + nx, y + ny), needle.rgb(nx, ny), threshold) {
                            continue 'candidate;
                        }
                    }
                }
                if !found(Point {
                    x: x as i64,
                    y: y as i64,
                }) {
                    return Ok(());
                }
            }
        }
        Ok(())
    }
    pub fn find_color(&self, color: &str, options: SearchOptions) -> Result<Option<Point>> {
        let mut first = None;
        self.scan_color(color, options, |p| {
            first = Some(p);
            false
        })?;
        Ok(first)
    }
    pub fn find_colors(&self, color: &str, options: SearchOptions) -> Result<Vec<Point>> {
        let mut points = vec![];
        self.scan_color(color, options, |p| {
            points.push(p);
            true
        })?;
        Ok(points)
    }
    pub fn count_color(&self, color: &str, options: SearchOptions) -> Result<usize> {
        let mut count = 0;
        self.scan_color(color, options, |_| {
            count += 1;
            true
        })?;
        Ok(count)
    }
    pub fn find_image(&self, needle: &Bitmap, options: SearchOptions) -> Result<Option<Point>> {
        let mut first = None;
        self.scan_image(needle, options, |p| {
            first = Some(p);
            false
        })?;
        Ok(first)
    }
    pub fn find_images(&self, needle: &Bitmap, options: SearchOptions) -> Result<Vec<Point>> {
        let mut points = vec![];
        self.scan_image(needle, options, |p| {
            points.push(p);
            true
        })?;
        Ok(points)
    }
    pub fn count_image(&self, needle: &Bitmap, options: SearchOptions) -> Result<usize> {
        let mut count = 0;
        self.scan_image(needle, options, |_| {
            count += 1;
            true
        })?;
        Ok(count)
    }
}
