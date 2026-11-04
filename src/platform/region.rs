use crate::{Display, Rect};
/// ScreenCaptureKit can produce transparent edges for tiny source rectangles.
/// Capture a padded region (at least 64 logical pixels), then sample the requested
/// interior. At display edges, clamp to the actual display instead of resampling.
pub(super) fn padded(display: &Display, rect: Rect) -> Rect {
    fn axis(origin: i64, size: usize, start: i64, length: usize) -> (i64, usize) {
        let size = size as i128;
        let width = (length as i128 + 32).max(64).min(size);
        let offset = (start as i128 - origin as i128 - 16)
            .max(0)
            .min(size - width);
        ((origin as i128 + offset) as i64, width as usize)
    }
    let (x, width) = axis(display.x, display.width, rect.x, rect.width);
    let (y, height) = axis(display.y, display.height, rect.y, rect.height);
    Rect {
        x,
        y,
        width,
        height,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_contains_requested_pixels_at_edges_and_signed_origins() {
        for (origin, size) in [(-1920, 1920), (-23, 25), (0, 1920)] {
            let d = Display {
                id: 1,
                x: origin,
                y: -7,
                width: size,
                height: 80,
                is_main: true,
            };
            for x in 0..size {
                for width in [1, (size - x).min(40), size - x] {
                    let r = Rect {
                        x: origin + x as i64,
                        y: 0,
                        width,
                        height: 1,
                    };
                    let p = padded(&d, r);
                    assert!(
                        p.x >= d.x
                            && p.x <= r.x
                            && p.x + p.width as i64 >= r.x + r.width as i64
                            && p.x + p.width as i64 <= d.x + d.width as i64
                    );
                    assert!(p.y <= r.y && p.y + p.height as i64 > r.y);
                }
            }
        }
        let d = Display {
            id: 1,
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            is_main: true,
        };
        let p = padded(
            &d,
            Rect {
                x: 900,
                y: 500,
                width: 1,
                height: 1,
            },
        );
        assert_eq!((p.width, p.height), (64, 64));
    }
}
