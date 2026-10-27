//! Node-API input and bounded asynchronous Rust media workers.
use napi::{Error, Result};
use napi_derive::napi;
use rusty_robot::Robot;

fn native<T>(result: rusty_robot::Result<T>) -> Result<T> {
    result.map_err(|error| Error::from_reason(error.to_string()))
}

// f64 preserves the original JS value until validation, unlike N-API's coercing
// integer conversions. The core library supports native signed 32-bit positions.
fn coordinate(value: f64) -> Result<i64> {
    if !value.is_finite()
        || value.fract() != 0.0
        || value < i32::MIN as f64
        || value > i32::MAX as f64
    {
        return Err(Error::from_reason(
            "Expected a finite signed 32-bit integer",
        ));
    }
    Ok(value as i64)
}

#[napi(object)]
pub struct ScreenSize {
    pub width: u32,
    pub height: u32,
}
#[napi(object)]
pub struct Pointer {
    pub x: f64,
    pub y: f64,
}
#[napi(object)]
pub struct Permissions {
    pub accessibility: Option<bool>,
    pub screen_capture: Option<bool>,
}

#[napi]
pub fn get_screen_size() -> Result<ScreenSize> {
    let size = native(Robot.get_screen_size())?;
    Ok(ScreenSize {
        width: u32::try_from(size.width)
            .map_err(|_| Error::from_reason("Display width overflow"))?,
        height: u32::try_from(size.height)
            .map_err(|_| Error::from_reason("Display height overflow"))?,
    })
}
#[napi]
pub fn get_mouse_pos() -> Result<Pointer> {
    let point = native(Robot.get_mouse_pos())?;
    Ok(Pointer {
        x: point.x as f64,
        y: point.y as f64,
    })
}
#[napi]
pub fn get_permissions() -> Result<Permissions> {
    Ok(Permissions {
        accessibility: native(Robot.get_accessibility_permission())?,
        screen_capture: native(Robot.get_screen_capture_permission())?,
    })
}
#[napi]
pub fn move_mouse(x: f64, y: f64) -> Result<()> {
    native(Robot.move_mouse(coordinate(x)?, coordinate(y)?))
}
#[napi]
pub fn drag_mouse(x: f64, y: f64, button: String) -> Result<()> {
    native(Robot.drag_mouse(coordinate(x)?, coordinate(y)?, Some(&button)))
}
#[napi]
pub fn mouse_toggle(state: String, button: String) -> Result<()> {
    native(Robot.mouse_toggle(&state, Some(&button)))
}
#[napi]
pub fn scroll_mouse(x: f64, y: f64) -> Result<()> {
    native(Robot.scroll_mouse(coordinate(x)?, coordinate(y)?))
}
#[napi]
pub fn key_tap(key: String, modifiers: Option<Vec<String>>) -> Result<()> {
    let modifiers = modifiers.unwrap_or_default();
    native(Robot.key_tap(
        &key,
        &modifiers.iter().map(String::as_str).collect::<Vec<_>>(),
    ))
}
#[napi]
pub fn key_toggle(key: String, state: String, modifiers: Option<Vec<String>>) -> Result<()> {
    let modifiers = modifiers.unwrap_or_default();
    native(Robot.key_toggle(
        &key,
        &state,
        &modifiers.iter().map(String::as_str).collect::<Vec<_>>(),
    ))
}
#[napi]
pub fn type_string(text: String) -> Result<()> {
    native(Robot.type_string(&text))
}

#[napi]
pub fn set_keyboard_delay(ms: f64) -> Result<()> {
    let value = coordinate(ms)?;
    if value < 0 {
        return Err(Error::from_reason("Delay must be nonnegative"));
    }
    native(Robot.set_keyboard_delay(value as u64))
}
#[napi]
pub fn mouse_click(button: Option<String>) -> Result<()> {
    native(Robot.mouse_click(button.as_deref(), false))
}

use napi::{
    Env, Task,
    bindgen_prelude::{AsyncTask, Buffer},
};
use rusty_robot_media::{Format, Frame, Options};
use std::sync::atomic::{AtomicUsize, Ordering};

// Bound work before it enters libuv's queue. A live frame and an on-demand
// screenshot can coexist; further requests fail rather than building stale lag.
static PENDING_MEDIA: AtomicUsize = AtomicUsize::new(0);
struct MediaPermit;
impl MediaPermit {
    #[allow(deprecated)] // fetch_update supports the Rust 1.88 MSRV.
    fn acquire() -> Result<Self> {
        PENDING_MEDIA
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < 2).then_some(count + 1)
            })
            .map(|_| Self)
            .map_err(|_| {
                Error::from_reason("Rust frame queue is full; retry after the current frame")
            })
    }
}
impl Drop for MediaPermit {
    fn drop(&mut self) {
        PENDING_MEDIA.fetch_sub(1, Ordering::AcqRel);
    }
}
fn positive(value: f64) -> Result<u32> {
    let value = coordinate(value)?;
    if !(1..=16384).contains(&value) {
        return Err(Error::from_reason("Image dimension must be 1..16384"));
    }
    Ok(value as u32)
}
fn media_options(
    width: Option<f64>,
    quality: Option<f64>,
    format: Option<String>,
) -> Result<Options> {
    let quality = coordinate(quality.unwrap_or(85.0))?;
    if !(1..=100).contains(&quality) {
        return Err(Error::from_reason("JPEG quality must be 1..100"));
    }
    let format = match format.as_deref().unwrap_or("jpeg") {
        "jpeg" | "jpg" => Format::Jpeg,
        "png" => Format::Png,
        _ => return Err(Error::from_reason("Format must be jpeg or png")),
    };
    Ok(Options {
        width: width.map(positive).transpose()?,
        quality: quality as u8,
        format,
    })
}
enum MediaAction {
    Capture {
        previous: Option<String>,
        force: bool,
    },
    Resize {
        input: Vec<u8>,
        width: u32,
        height: Option<u32>,
    },
}
pub struct MediaTask {
    options: Options,
    action: MediaAction,
    _permit: MediaPermit,
}
#[napi(object)]
pub struct EncodedFrame {
    pub capture_ms: f64,
    pub convert_ms: f64,
    pub resize_ms: f64,
    pub hash_ms: f64,
    pub encode_ms: f64,
    pub data: Option<Buffer>,
    pub width: u32,
    pub height: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub fingerprint: String,
    pub changed: bool,
}
impl Task for MediaTask {
    type Output = Frame;
    type JsValue = EncodedFrame;
    fn compute(&mut self) -> Result<Frame> {
        match &self.action {
            MediaAction::Capture { previous, force } => {
                rusty_robot_media::capture(self.options, previous.as_deref(), *force)
            }
            MediaAction::Resize {
                input,
                width,
                height,
            } => rusty_robot_media::resize_encoded(input, *width, *height, self.options),
        }
        .map_err(Error::from_reason)
    }
    fn resolve(&mut self, _env: Env, frame: Frame) -> Result<EncodedFrame> {
        Ok(EncodedFrame {
            capture_ms: frame.capture_ms,
            convert_ms: frame.convert_ms,
            resize_ms: frame.resize_ms,
            hash_ms: frame.hash_ms,
            encode_ms: frame.encode_ms,
            changed: frame.data.is_some(),
            data: frame.data.map(Buffer::from),
            width: frame.width,
            height: frame.height,
            source_width: frame.source_width,
            source_height: frame.source_height,
            fingerprint: frame.fingerprint,
        })
    }
}
/// Capture + resize + hash + encode on a worker; no JS pixel work or temporary file.
#[napi]
pub fn capture_frame(
    width: Option<f64>,
    quality: Option<f64>,
    previous: Option<String>,
    force: Option<bool>,
    format: Option<String>,
) -> Result<AsyncTask<MediaTask>> {
    let options = media_options(width, quality, format)?;
    Ok(AsyncTask::new(MediaTask {
        options,
        action: MediaAction::Capture {
            previous,
            force: force.unwrap_or(false),
        },
        _permit: MediaPermit::acquire()?,
    }))
}
/// Decode/resize/re-encode JPEG or PNG on a bounded worker.
#[napi]
pub fn resize_image(
    input: Buffer,
    width: f64,
    height: Option<f64>,
    quality: Option<f64>,
    format: Option<String>,
) -> Result<AsyncTask<MediaTask>> {
    let options = media_options(Some(width), quality, format)?;
    if input.len() > 64 * 1024 * 1024 {
        return Err(Error::from_reason("Encoded input exceeds 64 MiB"));
    }
    let width = positive(width)?;
    let height = height.map(positive).transpose()?;
    let permit = MediaPermit::acquire()?;
    Ok(AsyncTask::new(MediaTask {
        options,
        action: MediaAction::Resize {
            input: input.to_vec(),
            width,
            height,
        },
        _permit: permit,
    }))
}
