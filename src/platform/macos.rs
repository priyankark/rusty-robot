//! CoreGraphics/CoreFoundation adaptation. Every Create/Copy object is released by RAII.
use super::Button;
use crate::{Bitmap, Display, Point, Rect, Result, error::err};
use std::{ffi::c_void, ptr};
#[path = "macos/modifiers.rs"]
mod modifiers;
type Ref = *mut c_void;
#[repr(C)]
#[derive(Clone, Copy)]
struct CgPoint {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CgSize {
    width: f64,
    height: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CgRect {
    origin: CgPoint,
    size: CgSize,
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: Ref);
    fn CFDataGetBytePtr(value: Ref) -> *const u8;
    fn CFDataGetLength(value: Ref) -> isize;
    fn CFDictionaryCreate(
        allocator: Ref,
        keys: *const Ref,
        values: *const Ref,
        count: isize,
        key_callbacks: Ref,
        value_callbacks: Ref,
    ) -> Ref;
    static kCFBooleanTrue: Ref;
}
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: Ref) -> bool;
    static kAXTrustedCheckOptionPrompt: Ref;
}
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
    fn CGMainDisplayID() -> u32;
    fn CGGetActiveDisplayList(max: u32, ids: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayBounds(id: u32) -> CgRect;
    fn CGImageGetDataProvider(image: Ref) -> Ref;
    fn CGDataProviderCopyData(provider: Ref) -> Ref;
    fn CGImageGetBitsPerPixel(image: Ref) -> usize;
    fn CGImageGetWidth(image: Ref) -> usize;
    fn CGImageGetHeight(image: Ref) -> usize;
    fn CGImageGetBytesPerRow(image: Ref) -> usize;
    fn CGImageGetBitmapInfo(image: Ref) -> u32;
    static kCGColorSpaceSRGB: Ref;
    fn CGEventCreate(source: Ref) -> Ref;
    fn CGEventGetLocation(event: Ref) -> CgPoint;
    fn CGEventSetLocation(event: Ref, point: CgPoint);
    fn CGEventCreateMouseEvent(source: Ref, kind: u32, point: CgPoint, button: u32) -> Ref;
    fn CGEventSetIntegerValueField(event: Ref, field: u32, value: i64);
    fn CGEventPost(tap: u32, event: Ref);
    fn CGEventCreateScrollWheelEvent2(
        source: Ref,
        units: u32,
        count: u32,
        wheel1: i32,
        wheel2: i32,
        wheel3: i32,
    ) -> Ref;
    fn CGEventCreateKeyboardEvent(source: Ref, code: u16, down: bool) -> Ref;
    fn CGEventSetFlags(event: Ref, flags: u64);
    fn CGEventSetType(event: Ref, kind: u32);
    fn CGEventKeyboardSetUnicodeString(event: Ref, len: usize, text: *const u16);
}
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn TISCopyCurrentKeyboardLayoutInputSource() -> Ref;
    fn TISGetInputSourceProperty(source: Ref, property: Ref) -> Ref;
    static kTISPropertyUnicodeKeyLayoutData: Ref;
    fn LMGetKbdType() -> u8;
    fn UCKeyTranslate(
        layout: *const u8,
        code: u16,
        action: u16,
        modifiers: u32,
        keyboard: u32,
        options: u32,
        dead: *mut u32,
        max: usize,
        len: *mut usize,
        chars: *mut u16,
    ) -> i32;
}
struct Owned(Ref);
impl Owned {
    fn new(value: Ref) -> Result<Self> {
        if value.is_null() {
            Err(err(
                "macOS could not create a native object; check desktop permissions",
            ))
        } else {
            Ok(Self(value))
        }
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        // SAFETY: Owned contains exactly one non-null Create/Copy reference.
        unsafe { CFRelease(self.0) }
    }
}
pub(crate) fn accessibility(request: bool) -> Result<Option<bool>> {
    // SAFETY: Static CF objects outlive the synchronous AX call; the dictionary borrows them.
    unsafe {
        if !request {
            return Ok(Some(AXIsProcessTrusted()));
        }
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = Owned::new(CFDictionaryCreate(
            ptr::null_mut(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            ptr::null_mut(),
            ptr::null_mut(),
        ))?;
        Ok(Some(AXIsProcessTrustedWithOptions(options.0)))
    }
}
pub(crate) fn screen_permission(request: bool) -> Result<Option<bool>> {
    // SAFETY: These APIs take no pointers and return the current process's permission.
    Ok(Some(unsafe {
        if request {
            CGRequestScreenCaptureAccess()
        } else {
            CGPreflightScreenCaptureAccess()
        }
    }))
}
fn input_permission() -> Result<()> {
    if accessibility(false)? != Some(true) {
        Err(err(
            "Accessibility permission is required to inject desktop input",
        ))
    } else {
        Ok(())
    }
}
pub(crate) fn mouse_position() -> Result<Point> {
    // SAFETY: Event is owned for the duration of the read.
    unsafe {
        let event = Owned::new(CGEventCreate(ptr::null_mut()))?;
        let p = CGEventGetLocation(event.0);
        Ok(Point {
            x: p.x.round() as i64,
            y: p.y.round() as i64,
        })
    }
}
fn button_id(button: Button) -> u32 {
    match button {
        Button::Left => 0,
        Button::Right => 1,
        Button::Middle => 2,
    }
}
pub(crate) fn move_to(x: i32, y: i32, drag: Option<Button>) -> Result<()> {
    input_permission()?;
    let old = mouse_position()?;
    let (kind, button) = match drag {
        None => (5, Button::Left),
        Some(Button::Left) => (6, Button::Left),
        Some(Button::Right) => (7, Button::Right),
        Some(Button::Middle) => (27, Button::Middle),
    };
    // SAFETY: The event is owned and posted before its reference is released.
    unsafe {
        let event = Owned::new(CGEventCreateMouseEvent(
            ptr::null_mut(),
            kind,
            CgPoint {
                x: x as f64,
                y: y as f64,
            },
            button_id(button),
        ))?;
        CGEventSetIntegerValueField(event.0, 4, x as i64 - old.x);
        CGEventSetIntegerValueField(event.0, 5, y as i64 - old.y);
        CGEventPost(1, event.0);
    }
    Ok(())
}
pub(crate) fn button(button: Button, down: bool, clicks: i64) -> Result<()> {
    input_permission()?;
    let p = mouse_position()?;
    let kind = match (button, down) {
        (Button::Left, true) => 1,
        (Button::Left, false) => 2,
        (Button::Right, true) => 3,
        (Button::Right, false) => 4,
        (Button::Middle, true) => 25,
        (Button::Middle, false) => 26,
    };
    // SAFETY: The event parameters are valid CoreGraphics constants; lifetime is scoped.
    unsafe {
        let event = Owned::new(CGEventCreateMouseEvent(
            ptr::null_mut(),
            kind,
            CgPoint {
                x: p.x as f64,
                y: p.y as f64,
            },
            button_id(button),
        ))?;
        CGEventSetIntegerValueField(event.0, 1, clicks);
        CGEventPost(1, event.0);
    }
    Ok(())
}
pub(crate) fn scroll(x: i32, y: i32) -> Result<()> {
    input_permission()?;
    let location = mouse_position()?;
    // SAFETY: Use the non-variadic API to preserve wheel ordering on ARM64. Explicit location
    // keeps the scroll over the same control as the preceding pointer movement.
    unsafe {
        let event = Owned::new(CGEventCreateScrollWheelEvent2(
            ptr::null_mut(),
            0,
            2,
            y,
            x,
            0,
        ))?;
        CGEventSetLocation(
            event.0,
            CgPoint {
                x: location.x as f64,
                y: location.y as f64,
            },
        );
        CGEventPost(1, event.0);
    }
    Ok(())
}
pub(crate) fn key_code(key: &str) -> Result<u32> {
    let code = match key {
        "backspace" => 51,
        "delete" => 117,
        "enter" => 36,
        "tab" => 48,
        "escape" => 53,
        "up" => 126,
        "down" => 125,
        "right" => 124,
        "left" => 123,
        "home" => 115,
        "end" => 119,
        "pageup" => 116,
        "pagedown" => 121,
        "capslock" => 57,
        "command" => 55,
        "alt" => 58,
        "right_alt" => 61,
        "control" | "left_control" => 59,
        "right_control" => 62,
        "shift" => 56,
        "right_shift" => 60,
        "space" => 49,
        "f1" => 122,
        "f2" => 120,
        "f3" => 99,
        "f4" => 118,
        "f5" => 96,
        "f6" => 97,
        "f7" => 98,
        "f8" => 100,
        "f9" => 101,
        "f10" => 109,
        "f11" => 103,
        "f12" => 111,
        "f13" => 105,
        "f14" => 107,
        "f15" => 113,
        "f16" => 106,
        "f17" => 64,
        "f18" => 79,
        "f19" => 80,
        "f20" => 90,
        "numpad_0" => 82,
        "numpad_1" => 83,
        "numpad_2" => 84,
        "numpad_3" => 85,
        "numpad_4" => 86,
        "numpad_5" => 87,
        "numpad_6" => 88,
        "numpad_7" => 89,
        "numpad_8" => 91,
        "numpad_9" => 92,
        "numpad_+" => 69,
        "numpad_-" => 78,
        "numpad_*" => 67,
        "numpad_/" => 75,
        "numpad_." => 65,
        "numpad_lock" => 71,
        _ => return character_key(key),
    };
    Ok(code)
}
fn character_key(key: &str) -> Result<u32> {
    if key.len() != 1 || !key.is_ascii() {
        return Err(err("Invalid key code specified."));
    }
    let expected = key.as_bytes()[0] as u16;
    // SAFETY: The copied input source owns the borrowed layout data throughout translation.
    unsafe {
        let source = Owned::new(TISCopyCurrentKeyboardLayoutInputSource())?;
        let data = TISGetInputSourceProperty(source.0, kTISPropertyUnicodeKeyLayoutData);
        if data.is_null() {
            return Err(err("Current keyboard layout has no Unicode key map"));
        }
        let layout = CFDataGetBytePtr(data);
        if layout.is_null() {
            return Err(err("Current keyboard layout has no Unicode key map"));
        }
        for code in 0..128 {
            let mut dead = 0;
            let mut len = 0;
            let mut chars = [0; 4];
            if UCKeyTranslate(
                layout,
                code,
                3,
                0,
                LMGetKbdType() as u32,
                1,
                &mut dead,
                chars.len(),
                &mut len,
                chars.as_mut_ptr(),
            ) == 0
                && len == 1
                && chars[0] == expected
            {
                return Ok(code as u32);
            }
        }
    }
    Err(err("Invalid key code specified."))
}
static HELD_MODIFIERS: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
pub(crate) fn key_event(code: u32, down: bool, flags: u32) -> Result<()> {
    input_permission()?;
    use std::sync::atomic::Ordering;
    // Quartz requires flags on each event even when a preceding call held a
    // modifier physically down. The shared input lock serializes these updates.
    let (held, mut native) =
        modifiers::transition(HELD_MODIFIERS.load(Ordering::Relaxed), code, down);
    for (bit, mask) in [(1, 1 << 17), (2, 1 << 18), (4, 1 << 19), (8, 1 << 20)] {
        if flags & bit != 0 {
            native |= mask;
        }
    }
    if (123..=126).contains(&code) {
        native |= 1 << 23;
    }
    // SAFETY: Code is produced by key_code; event lifetime is owned through posting.
    unsafe {
        let event = Owned::new(CGEventCreateKeyboardEvent(
            ptr::null_mut(),
            code as u16,
            down,
        ))?;
        CGEventSetType(event.0, if down { 10 } else { 11 });
        CGEventSetFlags(event.0, native);
        CGEventPost(1, event.0);
        HELD_MODIFIERS.store(held, Ordering::Relaxed);
    }
    Ok(())
}
pub(crate) fn unicode(character: char) -> Result<()> {
    input_permission()?;
    let mut buffer = [0; 2];
    let utf16 = character.encode_utf16(&mut buffer);
    // SAFETY: UTF-16 is valid and borrowed only while the API copies it into the event.
    unsafe {
        for down in [true, false] {
            let event = Owned::new(CGEventCreateKeyboardEvent(ptr::null_mut(), 0, down))?;
            CGEventSetFlags(event.0, 0);
            CGEventKeyboardSetUnicodeString(event.0, utf16.len(), utf16.as_ptr());
            CGEventPost(1, event.0);
        }
    }
    Ok(())
}
pub(crate) fn displays() -> Result<Vec<Display>> {
    // SAFETY: CG receives the exact length of the allocated ID array.
    unsafe {
        let mut count = 0;
        if CGGetActiveDisplayList(0, ptr::null_mut(), &mut count) != 0 {
            return Err(err("Could not enumerate displays"));
        }
        let mut ids = vec![0; count as usize];
        if CGGetActiveDisplayList(count, ids.as_mut_ptr(), &mut count) != 0 {
            return Err(err("Could not enumerate displays"));
        }
        ids.truncate(count as usize);
        let main = CGMainDisplayID();
        Ok(ids
            .into_iter()
            .map(|id| {
                let b = CGDisplayBounds(id);
                Display {
                    id: id as u64,
                    x: b.origin.x as i64,
                    y: b.origin.y as i64,
                    width: b.size.width as usize,
                    height: b.size.height as usize,
                    is_main: id == main,
                }
            })
            .collect())
    }
}
pub(crate) fn capture(display: &Display, rect: Rect) -> Result<Bitmap> {
    use block2::RcBlock;
    use objc2::{AnyThread, ClassType, msg_send, sel};
    use objc2_core_graphics::CGImage;
    use objc2_foundation::{NSArray, NSError, NSPoint, NSRect, NSSize};
    use objc2_screen_capture_kit::{
        SCContentFilter, SCScreenshotManager, SCShareableContent, SCStreamConfiguration,
    };
    if screen_permission(false)? != Some(true) {
        return Err(err(
            "Screen Recording permission is required to capture the desktop",
        ));
    }
    // SAFETY: Check method availability before using the macOS 14 screenshot API.
    let available: bool = unsafe {
        msg_send![SCScreenshotManager::class(),respondsToSelector:sel!(captureImageWithFilter:configuration:completionHandler:)]
    };
    if !available {
        return Err(err("Screen capture requires macOS 14 or later"));
    }
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let display_id = display.id;
    let display_size = (display.width, display.height);
    let origin = (display.x, display.y);
    let content_ready = RcBlock::new(
        move |content: *mut SCShareableContent, error: *mut NSError| {
            if !error.is_null() {
                let _ = sender.send(Err(err(unsafe { &*error }
                    .localizedDescription()
                    .to_string())));
                return;
            }
            if content.is_null() {
                let _ = sender.send(Err(err("ScreenCaptureKit returned no shareable content")));
                return;
            }
            // SAFETY: Callback objects are borrowed within their live callback. Screenshot
            // API retains its filter/configuration and copies the nested completion block.
            unsafe {
                let displays = (&*content).displays();
                let Some(display) = displays.iter().find(|d| d.displayID() as u64 == display_id)
                else {
                    let _ = sender.send(Err(err("Display disappeared during capture")));
                    return;
                };
                let filter = SCContentFilter::initWithDisplay_excludingWindows(
                    SCContentFilter::alloc(),
                    &display,
                    &NSArray::new(),
                );
                let config = SCStreamConfiguration::new();
                let scale = filter.pointPixelScale() as f64;
                config.setWidth((display_size.0 as f64 * scale).round() as usize);
                config.setHeight((display_size.1 as f64 * scale).round() as usize);
                config.setSourceRect(NSRect::new(
                    NSPoint::new(0.0, 0.0),
                    NSSize::new(display_size.0 as f64, display_size.1 as f64),
                ));
                config.setShowsCursor(false);
                config.setPixelFormat(u32::from_be_bytes(*b"BGRA"));
                config.setColorSpaceName(&*kCGColorSpaceSRGB.cast());
                let sender = sender.clone();
                let completion = RcBlock::new(move |image: *mut CGImage, error: *mut NSError| {
                    let result = if !error.is_null() {
                        Err(err((&*error).localizedDescription().to_string()))
                    } else if image.is_null() {
                        Err(err("ScreenCaptureKit returned no image"))
                    } else {
                        copy_captured_image(image.cast(), rect, origin, display_size)
                    };
                    let _ = sender.send(result);
                });
                SCScreenshotManager::captureImageWithFilter_configuration_completionHandler(
                    &filter,
                    &config,
                    Some(&completion),
                );
            }
        },
    );
    // SAFETY: The callback captures owned values and can outlive this call on timeout.
    unsafe {
        SCShareableContent::getShareableContentExcludingDesktopWindows_onScreenWindowsOnly_completionHandler(true,true,&content_ready);
    }
    receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| err("ScreenCaptureKit screenshot timed out"))?
}
fn copy_captured_image(
    image: Ref,
    rect: Rect,
    origin: (i64, i64),
    display_size: (usize, usize),
) -> Result<Bitmap> {
    // Capture at native density to avoid resampling/antialiasing exact pixel colors.
    // Crop after capture because tiny ScreenCaptureKit source rectangles can acquire
    // partially transparent edges. Output is already sRGB/BGRA: no second conversion.
    unsafe {
        let width = CGImageGetWidth(image);
        let height = CGImageGetHeight(image);
        let stride = CGImageGetBytesPerRow(image);
        let bits = CGImageGetBitsPerPixel(image);
        let info = CGImageGetBitmapInfo(image);
        if display_size.0 == 0 || display_size.1 == 0 {
            return Err(err("Invalid display dimensions"));
        }
        let scale_x = width as f64 / display_size.0 as f64;
        let scale_y = height as f64 / display_size.1 as f64;
        let x = ((rect.x - origin.0) as f64 * scale_x).round() as usize;
        let y = ((rect.y - origin.1) as f64 * scale_y).round() as usize;
        let crop_width = (rect.width as f64 * scale_x).round() as usize;
        let crop_height = (rect.height as f64 * scale_y).round() as usize;
        if bits != 32
            || info & 0x7000 != 0x2000
            || !matches!(info & 0x1f, 2 | 6)
            || x.checked_add(crop_width).is_none_or(|end| end > width)
            || y.checked_add(crop_height).is_none_or(|end| end > height)
        {
            return Err(err("Unexpected ScreenCaptureKit bitmap format or size"));
        }
        let data = Owned::new(CGDataProviderCopyData(CGImageGetDataProvider(image)))?;
        let length = CFDataGetLength(data.0);
        if length < 0
            || stride
                .checked_mul(height)
                .is_none_or(|size| size > length as usize)
            || width.checked_mul(4).is_none_or(|row| row > stride)
        {
            return Err(err("Invalid ScreenCaptureKit bitmap buffer"));
        }
        let bytes = std::slice::from_raw_parts(CFDataGetBytePtr(data.0), length as usize);
        // RobotJS returns logical-size captures on macOS. Sample the center of
        // each logical pixel with nearest-neighbor selection, rather than letting
        // ScreenCaptureKit's downscaler blend adjacent UI colors at Retina edges.
        let mut bitmap = Bitmap::empty(rect.width, rect.height)?;
        for row in 0..rect.height {
            let source_y = y + ((row as f64 + 0.5) * scale_y).floor() as usize;
            for column in 0..rect.width {
                let source_x = x + ((column as f64 + 0.5) * scale_x).floor() as usize;
                let source = source_y.min(height - 1) * stride + source_x.min(width - 1) * 4;
                let dest = row * bitmap.byte_width + column * 4;
                bitmap.image[dest..dest + 4].copy_from_slice(&bytes[source..source + 4]);
            }
        }
        Ok(bitmap)
    }
}
