use crate::{Bitmap, Result, error::err};
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
};
pub(super) fn load(path: &Path) -> Result<Bitmap> {
    let mut decoder = png::Decoder::new(BufReader::new(File::open(path)?));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| err(e.to_string()))?;
    let len = reader
        .output_buffer_size()
        .ok_or_else(|| err("PNG image size overflow"))?;
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(len)
        .map_err(|e| err(e.to_string()))?;
    buffer.resize(len, 0);
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|e| err(e.to_string()))?;
    let mut bitmap = Bitmap::empty(info.width as usize, info.height as usize)?;
    let channels = info.color_type.samples();
    for (source, dest) in buffer[..info.buffer_size()]
        .chunks_exact(channels)
        .zip(bitmap.image.as_chunks_mut::<4>().0.iter_mut())
    {
        let (r, g, b, a) = match info.color_type {
            png::ColorType::Rgb => (source[0], source[1], source[2], 255),
            png::ColorType::Rgba => (source[0], source[1], source[2], source[3]),
            png::ColorType::Grayscale => (source[0], source[0], source[0], 255),
            png::ColorType::GrayscaleAlpha => (source[0], source[0], source[0], source[1]),
            png::ColorType::Indexed => return Err(err("PNG palette was not expanded")),
        };
        // Preserve unpremultiplied RGB, including pixels with alpha=0.
        dest.copy_from_slice(&[b, g, r, a]);
    }
    Ok(bitmap)
}
pub(super) fn save(bitmap: &Bitmap, path: &Path) -> Result<()> {
    let width = u32::try_from(bitmap.width).map_err(|_| err("PNG width overflow"))?;
    let height = u32::try_from(bitmap.height).map_err(|_| err("PNG height overflow"))?;
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(path)?), width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| err(e.to_string()))?;
    let mut stream = writer.stream_writer().map_err(|e| err(e.to_string()))?;
    use std::io::Write;
    for y in 0..bitmap.height {
        for x in 0..bitmap.width {
            stream.write_all(&bitmap.rgb(x, y))?;
        }
    }
    stream.finish().map_err(|e| err(e.to_string()))?;
    writer.finish().map_err(|e| err(e.to_string()))?;
    Ok(())
}
