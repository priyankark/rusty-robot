#![allow(dead_code)]
use rusty_robot::{Bitmap, MouseOutput, Point, Result};

pub fn point(x: i64, y: i64) -> Point {
    Point { x, y }
}
pub fn bitmap(rows: &[&[&str]]) -> Bitmap {
    padded_bitmap(rows, rows[0].len() * 4, 4, 0)
}
pub fn padded_bitmap(rows: &[&[&str]], stride: usize, bpp: u16, fill: u8) -> Bitmap {
    let mut image = vec![fill; stride * rows.len()];
    for (y, row) in rows.iter().enumerate() {
        assert_eq!(row.len(), rows[0].len());
        for (x, hex) in row.iter().enumerate() {
            let rgb = u32::from_str_radix(hex, 16).unwrap();
            let offset = y * stride + x * usize::from(bpp);
            image[offset..offset + 3].copy_from_slice(&[
                rgb as u8,
                (rgb >> 8) as u8,
                (rgb >> 16) as u8,
            ]);
            if bpp == 4 {
                image[offset + 3] = 255;
            }
        }
    }
    Bitmap {
        width: rows[0].len(),
        height: rows.len(),
        byte_width: stride,
        bits_per_pixel: bpp * 8,
        bytes_per_pixel: bpp,
        image,
        screen_x: 0,
        screen_y: 0,
        scale_x: 1.0,
        scale_y: 1.0,
    }
}
pub fn error<T: std::fmt::Debug>(result: Result<T>, expected: &str) {
    let actual = result
        .expect_err("operation should reject invalid input")
        .to_string();
    assert!(
        actual.contains(expected),
        "expected {expected:?}, got {actual:?}"
    );
}
pub fn hex(value: &str) {
    assert_eq!(value.len(), 6);
    assert!(value.bytes().all(|c| c.is_ascii_hexdigit()));
}
#[derive(Default)]
pub struct MouseSpy {
    pub moves: Vec<Point>,
    pub clicks: Vec<(Option<String>, bool)>,
}
impl MouseOutput for MouseSpy {
    fn move_mouse(&mut self, x: i64, y: i64) -> Result<()> {
        self.moves.push(point(x, y));
        Ok(())
    }
    fn mouse_click(&mut self, button: Option<&str>, double: bool) -> Result<()> {
        self.clicks.push((button.map(str::to_owned), double));
        Ok(())
    }
}
pub fn bmp_bytes(rows: &[&[&str]]) -> Vec<u8> {
    let width = rows[0].len();
    let height = rows.len();
    let stride = (width * 3 + 3) & !3;
    let mut data = vec![0; 54 + stride * height];
    data[0..2].copy_from_slice(b"BM");
    for (offset, value) in [
        (2, data.len()),
        (10, 54),
        (14, 40),
        (18, width),
        (22, height),
        (34, stride * height),
    ] {
        data[offset..offset + 4].copy_from_slice(&(value as u32).to_le_bytes());
    }
    data[26..28].copy_from_slice(&1u16.to_le_bytes());
    data[28..30].copy_from_slice(&24u16.to_le_bytes());
    let flipped: Vec<&[&str]> = rows.iter().rev().copied().collect();
    data[54..].copy_from_slice(&padded_bitmap(&flipped, stride, 3, 0).image);
    data
}
pub fn png_bytes(
    color_type: u8,
    rows: &[&[u8]],
    palette: Option<&[u8]>,
    transparency: Option<&[u8]>,
) -> Vec<u8> {
    use std::io::Write;
    fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = out.len();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let crc = crc32fast::hash(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = vec![];
    header.extend_from_slice(&2u32.to_be_bytes());
    header.extend_from_slice(&2u32.to_be_bytes());
    header.extend_from_slice(&[8, color_type, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);
    if let Some(p) = palette {
        chunk(&mut out, b"PLTE", p);
    }
    if let Some(t) = transparency {
        chunk(&mut out, b"tRNS", t);
    }
    let mut zlib = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    for row in rows {
        zlib.write_all(&[0]).unwrap();
        zlib.write_all(row).unwrap();
    }
    chunk(&mut out, b"IDAT", &zlib.finish().unwrap());
    chunk(&mut out, b"IEND", &[]);
    out
}
