//! File codecs. PNG support is selected at compile time.
mod bmp;
#[cfg(feature = "png")]
mod png_codec;
use crate::{Bitmap, Result, error::err};
use std::path::Path;
pub const SUPPORTS_PNG: bool = cfg!(feature = "png");
/// Encode in memory; fast=true trades compressed size for latency.
/// For resize/JPEG and coordinate-preserving frames use the optional media module.
#[cfg(feature = "png")]
pub fn encode_png(bitmap: &Bitmap, fast: bool) -> Result<Vec<u8>> {
    png_codec::bytes(bitmap, fast)
}
fn extension(path: &Path) -> Result<String> {
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match extension.as_str() {
        "bmp" => Ok(extension),
        "png" if SUPPORTS_PNG => Ok(extension),
        "png" => Err(err("PNG support is not enabled")),
        _ => Err(err("Unsupported image type")),
    }
}
pub fn load(path: &Path) -> Result<Bitmap> {
    match extension(path)?.as_str() {
        "bmp" => bmp::load(path),
        #[cfg(feature = "png")]
        "png" => png_codec::load(path),
        _ => Err(err("Unsupported image type")),
    }
}
pub fn save(bitmap: &Bitmap, path: &Path) -> Result<()> {
    let extension = extension(path)?;
    bitmap.validate()?;
    match extension.as_str() {
        "bmp" => bmp::save(bitmap, path),
        #[cfg(feature = "png")]
        "png" => png_codec::save(bitmap, path),
        _ => Err(err("Unsupported image type")),
    }
}
