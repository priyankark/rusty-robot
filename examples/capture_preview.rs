//! Save a resized JPEG and map preview coordinates back to the desktop.
//! cargo run --features media --example capture_preview -- preview.jpg
use rusty_robot::{Point, media};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("Usage: capture_preview OUTPUT.jpg")?;
    let frame = media::capture(media::Options::default(), None, true)?;
    let bytes = frame.data.as_ref().ok_or("No encoded frame returned")?;
    std::fs::write(&path, bytes)?;
    let center = Point {
        x: i64::from(frame.width / 2),
        y: i64::from(frame.height / 2),
    };
    println!(
        "Saved {}×{} JPEG to {}",
        frame.width,
        frame.height,
        std::path::Path::new(&path).display()
    );
    println!(
        "Preview center {center:?} maps to {:?}",
        frame.to_screen_point(center)?
    );
    Ok(())
}
