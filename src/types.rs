use crate::Result;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Size {
    pub width: usize,
    pub height: usize,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: i64,
    pub y: i64,
    pub width: usize,
    pub height: usize,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct SearchOptions {
    pub x: Option<usize>,
    pub y: Option<usize>,
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub tolerance: f64,
}
#[derive(Debug, Clone)]
pub struct Display {
    pub id: u64,
    pub x: i64,
    pub y: i64,
    pub width: usize,
    pub height: usize,
    pub is_main: bool,
}

/// Raw owned bitmap. Operations must validate metadata before accessing the buffer.
#[derive(Debug, Clone)]
pub struct Bitmap {
    pub width: usize,
    pub height: usize,
    pub byte_width: usize,
    pub bits_per_pixel: u16,
    pub bytes_per_pixel: u16,
    pub image: Vec<u8>,
    pub screen_x: i64,
    pub screen_y: i64,
    pub scale_x: f64,
    pub scale_y: f64,
}
pub type Image = Bitmap;

/// Injection seam for verifying image-click side effects without desktop access.
pub trait MouseOutput {
    fn move_mouse(&mut self, x: i64, y: i64) -> Result<()>;
    fn mouse_click(&mut self, button: Option<&str>, double: bool) -> Result<()>;
}
