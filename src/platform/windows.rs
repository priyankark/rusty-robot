//! Win32 SendInput and GDI capture, with scoped DPI context and GDI handles.
use super::Button;
use crate::{Bitmap, Display, Point, Rect, Result, error::err};
use std::{mem::size_of, ptr};
use windows_sys::Win32::{
    Foundation::{LPARAM, POINT, RECT},
    Graphics::Gdi::*,
    UI::{HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};
fn failure(context: &str) -> crate::Error {
    err(format!("{context}: {}", std::io::Error::last_os_error()))
}
struct Dpi(DPI_AWARENESS_CONTEXT);
impl Dpi {
    fn enter() -> Self {
        // SAFETY: Changes only the calling thread; Drop restores it.
        Self(unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) })
    }
}
impl Drop for Dpi {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                SetThreadDpiAwarenessContext(self.0);
            }
        }
    }
}
pub(crate) fn accessibility(_: bool) -> Result<Option<bool>> {
    Ok(None)
}
pub(crate) fn screen_permission(_: bool) -> Result<Option<bool>> {
    Ok(None)
}
fn send(input: INPUT) -> Result<()> {
    // SAFETY: The initialized union arm matches INPUT.type, and the slice lives through the call.
    if unsafe { SendInput(1, &input, size_of::<INPUT>() as i32) } != 1 {
        return Err(failure("SendInput failed (check target privilege level)"));
    }
    Ok(())
}
fn mouse(flags: u32, x: i32, y: i32, data: u32) -> Result<()> {
    send(INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: x,
                dy: y,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    })
}
pub(crate) fn mouse_position() -> Result<Point> {
    let _dpi = Dpi::enter();
    let mut p = POINT::default();
    // SAFETY: p points to a writable POINT.
    if unsafe { GetCursorPos(&mut p) } == 0 {
        return Err(failure("GetCursorPos"));
    }
    Ok(Point {
        x: p.x as i64,
        y: p.y as i64,
    })
}
pub(crate) fn move_to(x: i32, y: i32, _drag: Option<Button>) -> Result<()> {
    let _dpi = Dpi::enter();
    // SAFETY: These metrics do not take pointers.
    let (left, top, width, height) = unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    };
    if width <= 0 || height <= 0 {
        return Err(err("No Windows desktop is available"));
    }
    let absolute = |value: i32, origin: i32, size: i32| {
        (((value as i64 - origin as i64) * 65536 + 32768) / size as i64).clamp(0, 65535) as i32
    };
    mouse(
        MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
        absolute(x, left, width),
        absolute(y, top, height),
        0,
    )
}
pub(crate) fn button(button: Button, down: bool, _clicks: i64) -> Result<()> {
    let flag = match (button, down) {
        (Button::Left, true) => MOUSEEVENTF_LEFTDOWN,
        (Button::Left, false) => MOUSEEVENTF_LEFTUP,
        (Button::Right, true) => MOUSEEVENTF_RIGHTDOWN,
        (Button::Right, false) => MOUSEEVENTF_RIGHTUP,
        (Button::Middle, true) => MOUSEEVENTF_MIDDLEDOWN,
        (Button::Middle, false) => MOUSEEVENTF_MIDDLEUP,
    };
    mouse(flag, 0, 0, 0)
}
pub(crate) fn scroll(x: i32, y: i32) -> Result<()> {
    if y != 0 {
        mouse(MOUSEEVENTF_WHEEL, 0, 0, y as u32)?;
    }
    if x != 0 {
        mouse(MOUSEEVENTF_HWHEEL, 0, 0, x.wrapping_neg() as u32)?;
    }
    Ok(())
}
pub(crate) fn key_code(key: &str) -> Result<u32> {
    if key.len() == 1 && key.is_ascii() {
        // SAFETY: ASCII is one valid UTF-16 code unit.
        let code = unsafe { VkKeyScanW(key.as_bytes()[0] as u16) };
        return if code == -1 {
            Err(err("Invalid key code specified."))
        } else {
            Ok(code as u16 as u32)
        };
    }
    if let Some(number) = key
        .strip_prefix('f')
        .and_then(|n| n.parse::<u16>().ok())
        .filter(|n| (1..=24).contains(n))
    {
        return Ok((VK_F1 + number - 1) as u32);
    }
    if let Some(number) = key
        .strip_prefix("numpad_")
        .and_then(|n| n.parse::<u16>().ok())
        .filter(|n| *n <= 9)
    {
        return Ok((VK_NUMPAD0 + number) as u32);
    }
    Ok(match key {
        "backspace" => VK_BACK,
        "delete" => VK_DELETE,
        "enter" => VK_RETURN,
        "tab" => VK_TAB,
        "escape" => VK_ESCAPE,
        "up" => VK_UP,
        "down" => VK_DOWN,
        "right" => VK_RIGHT,
        "left" => VK_LEFT,
        "home" => VK_HOME,
        "end" => VK_END,
        "pageup" => VK_PRIOR,
        "pagedown" => VK_NEXT,
        "capslock" => VK_CAPITAL,
        "command" => VK_LWIN,
        "alt" => VK_MENU,
        "right_alt" => VK_RMENU,
        "control" | "left_control" => VK_CONTROL,
        "right_control" => VK_RCONTROL,
        "shift" => VK_SHIFT,
        "right_shift" => VK_RSHIFT,
        "space" => VK_SPACE,
        "printscreen" => VK_SNAPSHOT,
        "insert" => VK_INSERT,
        "menu" => VK_APPS,
        "numpad_lock" => VK_NUMLOCK,
        "numpad_+" => VK_ADD,
        "numpad_-" => VK_SUBTRACT,
        "numpad_*" => VK_MULTIPLY,
        "numpad_/" => VK_DIVIDE,
        "numpad_." => VK_DECIMAL,
        "audio_mute" => VK_VOLUME_MUTE,
        "audio_vol_down" => VK_VOLUME_DOWN,
        "audio_vol_up" => VK_VOLUME_UP,
        "audio_play" | "audio_pause" => VK_MEDIA_PLAY_PAUSE,
        "audio_stop" => VK_MEDIA_STOP,
        "audio_prev" => VK_MEDIA_PREV_TRACK,
        "audio_next" => VK_MEDIA_NEXT_TRACK,
        _ => return Err(err("Invalid key code specified.")),
    } as u32)
}
fn raw_key(code: u16, down: bool) -> Result<()> {
    let extended = matches!(
        code,
        VK_RCONTROL
            | VK_RMENU
            | VK_INSERT
            | VK_DELETE
            | VK_HOME
            | VK_END
            | VK_PRIOR
            | VK_NEXT
            | VK_LEFT
            | VK_RIGHT
            | VK_UP
            | VK_DOWN
            | VK_LWIN
            | VK_RWIN
            | VK_APPS
            | VK_SNAPSHOT
            | VK_DIVIDE
    );
    let flags =
        if down { 0 } else { KEYEVENTF_KEYUP } | if extended { KEYEVENTF_EXTENDEDKEY } else { 0 };
    send(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: code,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    })
}
pub(crate) fn key_event(code: u32, down: bool, flags: u32) -> Result<()> {
    let implicit = (code >> 8) & !flags;
    let modifiers = [(1, VK_SHIFT), (2, VK_CONTROL), (4, VK_MENU)];
    if down {
        for (bit, key) in modifiers {
            if implicit & bit != 0 {
                raw_key(key, true)?;
            }
        }
    }
    let mut result = raw_key((code & 0xff) as u16, down);
    if !down || result.is_err() {
        for (bit, key) in modifiers.into_iter().rev() {
            if implicit & bit != 0 {
                let release = raw_key(key, false);
                if result.is_ok() {
                    result = release;
                }
            }
        }
    }
    result
}
pub(crate) fn unicode(character: char) -> Result<()> {
    let mut utf16 = [0; 2];
    for unit in character.encode_utf16(&mut utf16) {
        for down in [true, false] {
            send(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: 0,
                        wScan: *unit,
                        dwFlags: KEYEVENTF_UNICODE | if down { 0 } else { KEYEVENTF_KEYUP },
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            })?;
        }
    }
    Ok(())
}
pub(crate) fn displays() -> Result<Vec<Display>> {
    let _dpi = Dpi::enter();
    let mut displays = Vec::<Display>::new();
    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _dc: HDC,
        _rect: *mut RECT,
        data: LPARAM,
    ) -> i32 {
        // SAFETY: EnumDisplayMonitors synchronously passes our live Vec pointer.
        let displays = unsafe { &mut *(data as *mut Vec<Display>) };
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 || displays.try_reserve(1).is_err() {
            return 0;
        }
        displays.push(Display {
            id: monitor as usize as u64,
            x: info.rcMonitor.left as i64,
            y: info.rcMonitor.top as i64,
            width: (info.rcMonitor.right - info.rcMonitor.left) as usize,
            height: (info.rcMonitor.bottom - info.rcMonitor.top) as usize,
            is_main: info.dwFlags & 1 != 0,
        });
        1
    }
    // SAFETY: Callback does not unwind; the mutable Vec remains valid through enumeration.
    if unsafe {
        EnumDisplayMonitors(
            ptr::null_mut(),
            ptr::null(),
            Some(collect),
            &mut displays as *mut _ as LPARAM,
        )
    } == 0
    {
        return Err(failure("EnumDisplayMonitors"));
    }
    Ok(displays)
}
pub(crate) fn capture(_display: &Display, rect: Rect) -> Result<Bitmap> {
    let _dpi = Dpi::enter();
    let mut bitmap = Bitmap::empty(rect.width, rect.height)?;
    let width = i32::try_from(rect.width).map_err(|_| err("Capture width overflow"))?;
    let height = i32::try_from(rect.height).map_err(|_| err("Capture height overflow"))?;
    let x = super::coordinate(rect.x)?;
    let y = super::coordinate(rect.y)?;
    struct Resources {
        screen: HDC,
        memory: HDC,
        bitmap: HBITMAP,
        previous: HGDIOBJ,
    }
    impl Drop for Resources {
        fn drop(&mut self) {
            // SAFETY: Each non-null handle is owned; deselect before deletion.
            unsafe {
                if !self.previous.is_null() {
                    SelectObject(self.memory, self.previous);
                }
                if !self.bitmap.is_null() {
                    DeleteObject(self.bitmap);
                }
                if !self.memory.is_null() {
                    DeleteDC(self.memory);
                }
                if !self.screen.is_null() {
                    ReleaseDC(ptr::null_mut(), self.screen);
                }
            }
        }
    }
    // SAFETY: Dimensions are checked; DIB owns the buffer until it has been copied into Rust.
    unsafe {
        let mut handles = Resources {
            screen: GetDC(ptr::null_mut()),
            memory: ptr::null_mut(),
            bitmap: ptr::null_mut(),
            previous: ptr::null_mut(),
        };
        if handles.screen.is_null() {
            return Err(failure("GetDC"));
        }
        handles.memory = CreateCompatibleDC(handles.screen);
        if handles.memory.is_null() {
            return Err(failure("CreateCompatibleDC"));
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = ptr::null_mut();
        handles.bitmap = CreateDIBSection(
            handles.screen,
            &info,
            DIB_RGB_COLORS,
            &mut pixels,
            ptr::null_mut(),
            0,
        );
        if handles.bitmap.is_null() || pixels.is_null() {
            return Err(failure("CreateDIBSection"));
        }
        handles.previous = SelectObject(handles.memory, handles.bitmap);
        if handles.previous.is_null() || handles.previous as isize == -1 {
            handles.previous = ptr::null_mut();
            return Err(failure("SelectObject"));
        }
        if BitBlt(
            handles.memory,
            0,
            0,
            width,
            height,
            handles.screen,
            x,
            y,
            SRCCOPY | CAPTUREBLT,
        ) == 0
        {
            return Err(failure("BitBlt"));
        }
        if GdiFlush() == 0 {
            return Err(failure("GdiFlush"));
        }
        let len = bitmap.image.len();
        bitmap
            .image
            .copy_from_slice(std::slice::from_raw_parts(pixels.cast::<u8>(), len));
    }
    Ok(bitmap)
}
