//! Convert tagged native screenshots to owned sRGB BGRA pixels using one shared
//! Core Image context. The same color conversion applies to every capture size.
use super::*;
use objc2::{
    AnyThread,
    rc::{Retained, autoreleasepool},
};
use objc2_core_foundation::CGAffineTransform;
use objc2_core_graphics::{CGColorSpace, CGImage};
use objc2_core_image::{CIContext, CIImage, CIRenderDestination, kCIFormatBGRA8};
use std::sync::OnceLock;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGColorSpaceCreateWithName(name: Ref) -> Ref;

}

pub(super) fn render(image: Ref, width: usize, height: usize) -> Result<Bitmap> {
    autoreleasepool(|_| {
        if image.is_null() || width == 0 || height == 0 {
            return Err(err("Invalid screenshot image or dimensions"));
        }
        let mut bitmap = Bitmap::empty(width, height)?;
        // SAFETY: Caller borrows a live CGImage throughout rendering. Bitmap::empty
        // validates allocation/stride arithmetic; its allocation stays fixed until
        // the render task has finished (including its error path).
        unsafe {
            let source_width = CGImageGetWidth(image);
            let source_height = CGImageGetHeight(image);
            if source_width == 0 || source_height == 0 {
                return Err(err("ScreenCaptureKit returned an empty image"));
            }
            let space = Owned::new(CGColorSpaceCreateWithName(kCGColorSpaceSRGB))?;
            // Apple's CIContext and CIImage are explicitly thread-safe. Share
            // compiled kernels, not frames. Each request has a new source image
            // and destination buffer; no screenshot pixels are returned from a cache.
            static CONTEXT: OnceLock<Retained<CIContext>> = OnceLock::new();
            let context = CONTEXT.get_or_init(|| CIContext::contextWithOptions(None));
            let source =
                CIImage::imageWithCGImage(&*image.cast::<CGImage>()).imageBySamplingNearest();
            let scaled = source.imageByApplyingTransform(CGAffineTransform {
                a: width as f64 / source_width as f64,
                b: 0.0,
                c: 0.0,
                d: height as f64 / source_height as f64,
                tx: 0.0,
                ty: 0.0,
            });
            let destination =
                CIRenderDestination::initWithBitmapData_width_height_bytesPerRow_format(
                    CIRenderDestination::alloc(),
                    ptr::NonNull::new(bitmap.image.as_mut_ptr().cast()).unwrap(),
                    width,
                    height,
                    bitmap.byte_width,
                    kCIFormatBGRA8,
                );
            destination.setColorSpace(Some(&*space.0.cast::<CGColorSpace>()));
            destination.setFlipped(true); // First byte is the top-left pixel.
            destination.setDithered(false); // Stable colors for exact bitmap search.
            let task = context
                .startTaskToRender_toDestination_error(&scaled, &destination)
                .map_err(|e| err(e.localizedDescription().to_string()))?;
            task.waitUntilCompletedAndReturnError()
                .map_err(|e| err(e.localizedDescription().to_string()))?;
        }
        Ok(bitmap)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGBitmapContextCreate(
            data: Ref,
            width: usize,
            height: usize,
            bits: usize,
            stride: usize,
            color_space: Ref,
            info: u32,
        ) -> Ref;
        fn CGBitmapContextCreateImage(context: Ref) -> Ref;
    }

    fn pixel(x: usize, y: usize) -> [u8; 4] {
        [
            (x * 17 % 256) as u8,
            (y * 29 % 256) as u8,
            ((x + y) * 11 % 256) as u8,
            255,
        ]
    }
    fn source(width: usize, height: usize, _scale: usize) -> Owned {
        let mut data = vec![0; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                data[(y * width + x) * 4..(y * width + x + 1) * 4].copy_from_slice(&pixel(x, y));
            }
        }
        // SAFETY: Context borrows data only until it is dropped below. Creating
        // the image snapshots its storage, so the returned image owns its pixels.
        unsafe {
            let space = Owned::new(CGColorSpaceCreateWithName(kCGColorSpaceSRGB)).unwrap();
            let context = Owned::new(CGBitmapContextCreate(
                data.as_mut_ptr().cast(),
                width,
                height,
                8,
                width * 4,
                space.0,
                0x2002,
            ))
            .unwrap();
            Owned::new(CGBitmapContextCreateImage(context.0)).unwrap()
        }
    }
    fn verify(width: usize, height: usize, scale: usize) {
        let source = source(width * scale, height * scale, scale);
        let result = render(source.0, width, height).unwrap();
        result.validate().unwrap();
        for y in 0..height {
            for x in 0..width {
                let expected = pixel(x * scale + scale / 2, y * scale + scale / 2);
                let actual =
                    &result.image[y * result.byte_width + x * 4..y * result.byte_width + x * 4 + 4];
                assert_eq!(
                    actual, expected,
                    "color, orientation, alpha or nearest sampling at ({x},{y}) in {width}x{height}"
                );
            }
        }
    }
    #[test]
    fn small_images_preserve_srgb_orientation_and_nearest_samples() {
        verify(17, 13, 2);
        verify(1, 1, 2);
    }
    #[test]
    fn core_image_preserves_srgb_orientation_and_nearest_samples() {
        verify(256, 256, 2);
    }
    #[test]
    fn native_density_does_not_drop_pixels() {
        verify(257, 259, 1);
    }
    #[test]
    fn shared_context_handles_independent_concurrent_frames() {
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| verify(256, 256, 2));
            }
        });
    }
    #[test]
    fn rejects_invalid_dimensions_before_drawing() {
        assert!(render(ptr::null_mut(), 1, 1).is_err());
        let source = source(2, 2, 1);
        assert!(render(source.0, 0, 1).is_err());
        assert!(render(source.0, usize::MAX, 2).is_err());
    }
}
