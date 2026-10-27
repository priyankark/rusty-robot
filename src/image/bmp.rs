use crate::{Bitmap, Result, error::err};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::Path,
};

pub(super) fn load(path: &Path) -> Result<Bitmap> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut header = [0; 54];
    reader.read_exact(&mut header)?;
    let u32_at = |i| u32::from_le_bytes(header[i..i + 4].try_into().unwrap());
    let u16_at = |i| u16::from_le_bytes(header[i..i + 2].try_into().unwrap());
    if &header[..2] != b"BM" || u32_at(14) < 40 || u16_at(26) != 1 || u32_at(30) != 0 {
        return Err(err("Unsupported or invalid BMP header"));
    }
    let width = u32_at(18) as i32;
    let height = u32_at(22) as i32;
    let bits = u16_at(28);
    if width <= 0 || height == 0 || height == i32::MIN || !matches!(bits, 24 | 32) {
        return Err(err("Unsupported BMP dimensions or pixel format"));
    }
    let (width, rows) = (width as usize, height.unsigned_abs() as usize);
    let bpp = (bits / 8) as usize;
    let stride = width
        .checked_mul(bpp)
        .and_then(|v| v.checked_add(3))
        .map(|v| v & !3)
        .ok_or_else(|| err("BMP row size overflow"))?;
    let offset = u32_at(10) as u64;
    let header_end = 14 + u64::from(u32_at(14));
    let bytes = stride
        .checked_mul(rows)
        .ok_or_else(|| err("BMP image size overflow"))?;
    let file_len = reader.get_ref().metadata()?.len();
    if offset < header_end
        || offset
            .checked_add(bytes as u64)
            .is_none_or(|v| v > file_len)
    {
        return Err(err("BMP pixel data is truncated or overlaps its header"));
    }
    let mut bitmap = Bitmap::empty(width, rows)?;
    let mut row = Vec::new();
    if bpp == 3 {
        row.try_reserve_exact(stride)
            .map_err(|e| err(e.to_string()))?;
        row.resize(stride, 0);
    }
    reader.seek(SeekFrom::Start(offset))?;
    for file_y in 0..rows {
        let y = if height > 0 {
            rows - file_y - 1
        } else {
            file_y
        };
        let dest = &mut bitmap.image[y * bitmap.byte_width..(y + 1) * bitmap.byte_width];
        if bpp == 4 {
            // Like RobotJS, copy compatible BGR(A) rows without channel conversion.
            reader.read_exact(dest)?;
        } else {
            reader.read_exact(&mut row)?;
            for (source, pixel) in row[..width * 3]
                .as_chunks::<3>()
                .0
                .iter()
                .zip(dest.as_chunks_mut::<4>().0.iter_mut())
            {
                *pixel = [source[0], source[1], source[2], 255];
            }
        }
    }
    Ok(bitmap)
}
pub(super) fn save(bitmap: &Bitmap, path: &Path) -> Result<()> {
    let width =
        i32::try_from(bitmap.width).map_err(|_| err("BMP width is outside the supported range"))?;
    let height = i32::try_from(bitmap.height)
        .map_err(|_| err("BMP height is outside the supported range"))?;
    // 24-bit RGB is sufficient for RobotJS's color contract; row padding is zeroed.
    let stride = bitmap
        .width
        .checked_mul(3)
        .and_then(|v| v.checked_add(3))
        .map(|v| v & !3)
        .ok_or_else(|| err("BMP row size overflow"))?;
    let bytes = stride
        .checked_mul(bitmap.height)
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| err("BMP image size overflow"))?;
    let size = bytes
        .checked_add(54)
        .ok_or_else(|| err("BMP file size overflow"))?;
    let mut header = [0; 54];
    header[..2].copy_from_slice(b"BM");
    for (offset, value) in [
        (2, size),
        (10, 54),
        (14, 40),
        (18, width as u32),
        (22, height as u32),
        (34, bytes),
    ] {
        header[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    header[26..28].copy_from_slice(&1u16.to_le_bytes());
    header[28..30].copy_from_slice(&24u16.to_le_bytes());
    // Build the complete payload before one bulk write, as RobotJS does.
    // Keep the existing 24-bit output contract and deterministic zero padding.
    let mut data = Vec::new();
    data.try_reserve_exact(size as usize)
        .map_err(|e| err(e.to_string()))?;
    data.resize(size as usize, 0);
    data[..54].copy_from_slice(&header);
    for (file_y, y) in (0..bitmap.height).rev().enumerate() {
        let row = &mut data[54 + file_y * stride..54 + (file_y + 1) * stride];
        let bpp = bitmap.bytes_per_pixel as usize;
        let source =
            &bitmap.image[y * bitmap.byte_width..y * bitmap.byte_width + bitmap.width * bpp];
        if bpp == 3 {
            row[..source.len()].copy_from_slice(source);
        } else {
            for (pixel, dest) in source
                .as_chunks::<4>()
                .0
                .iter()
                .zip(row[..bitmap.width * 3].as_chunks_mut::<3>().0.iter_mut())
            {
                dest.copy_from_slice(&pixel[..3]);
            }
        }
    }
    let mut writer = BufWriter::new(File::create(path)?);
    writer.write_all(&data)?;
    writer.flush()?;
    Ok(())
}
