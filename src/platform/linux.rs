//! X11/XTest backend using the Rust protocol implementation (no libX11 FFI).
use super::Button;
use crate::{Bitmap, Display, Point, Rect, Result, error::err};
use x11rb::{
    connection::Connection,
    protocol::{
        randr::ConnectionExt as _,
        xproto::{self, ConnectionExt as _, ImageFormat, ImageOrder},
        xtest::ConnectionExt as _,
    },
    rust_connection::RustConnection,
};
fn failure(e: impl std::fmt::Display) -> crate::Error {
    err(format!("X11: {e}"))
}
fn connect() -> Result<(RustConnection, usize)> {
    x11rb::connect(None).map_err(|e| {
        err(format!(
            "Cannot connect to X11 (an X11 session and DISPLAY are required): {e}"
        ))
    })
}
pub(crate) fn accessibility(_: bool) -> Result<Option<bool>> {
    Ok(None)
}
pub(crate) fn screen_permission(_: bool) -> Result<Option<bool>> {
    Ok(None)
}
pub(crate) fn mouse_position() -> Result<Point> {
    let (conn, screen) = connect()?;
    let p = conn
        .query_pointer(conn.setup().roots[screen].root)
        .map_err(failure)?
        .reply()
        .map_err(failure)?;
    Ok(Point {
        x: p.root_x as i64,
        y: p.root_y as i64,
    })
}
pub(crate) fn move_to(x: i32, y: i32, _drag: Option<Button>) -> Result<()> {
    let (conn, screen) = connect()?;
    let x = i16::try_from(x).map_err(|_| err("X11 coordinate is outside the supported range"))?;
    let y = i16::try_from(y).map_err(|_| err("X11 coordinate is outside the supported range"))?;
    conn.warp_pointer(0u32, conn.setup().roots[screen].root, 0, 0, 0, 0, x, y)
        .map_err(failure)?
        .check()
        .map_err(failure)
}
fn fake(conn: &RustConnection, kind: u8, detail: u8) -> Result<()> {
    conn.xtest_fake_input(kind, detail, 0, 0, 0, 0, 0)
        .map_err(failure)?
        .check()
        .map_err(failure)
}
pub(crate) fn button(button: Button, down: bool, _clicks: i64) -> Result<()> {
    let (conn, _) = connect()?;
    let detail = match button {
        Button::Left => 1,
        Button::Middle => 2,
        Button::Right => 3,
    };
    fake(
        &conn,
        if down {
            xproto::BUTTON_PRESS_EVENT
        } else {
            xproto::BUTTON_RELEASE_EVENT
        },
        detail,
    )
}
pub(crate) fn scroll(x: i32, y: i32) -> Result<()> {
    let (conn, _) = connect()?;
    for (amount, positive, negative) in [(y, 4, 5), (x, 6, 7)] {
        for _ in 0..amount.unsigned_abs() {
            let detail = if amount >= 0 { positive } else { negative };
            fake(&conn, xproto::BUTTON_PRESS_EVENT, detail)?;
            fake(&conn, xproto::BUTTON_RELEASE_EVENT, detail)?;
        }
    }
    Ok(())
}
pub(crate) fn key_code(key: &str) -> Result<u32> {
    if key.len() == 1 && key.is_ascii() {
        return Ok(key.as_bytes()[0] as u32);
    }
    if let Some(number) = key
        .strip_prefix('f')
        .and_then(|n| n.parse::<u32>().ok())
        .filter(|n| (1..=24).contains(n))
    {
        return Ok(0xffbd + number);
    }
    Ok(match key {
        "backspace" => 0xff08,
        "delete" => 0xffff,
        "enter" => 0xff0d,
        "tab" => 0xff09,
        "escape" => 0xff1b,
        "up" => 0xff52,
        "down" => 0xff54,
        "right" => 0xff53,
        "left" => 0xff51,
        "home" => 0xff50,
        "end" => 0xff57,
        "pageup" => 0xff55,
        "pagedown" => 0xff56,
        "capslock" => 0xffe5,
        "command" => 0xffeb,
        "alt" => 0xffe9,
        "right_alt" => 0xffea,
        "control" | "left_control" => 0xffe3,
        "right_control" => 0xffe4,
        "shift" => 0xffe1,
        "right_shift" => 0xffe2,
        "space" => 0x20,
        "printscreen" => 0xff61,
        "insert" => 0xff63,
        "menu" => 0xff67,
        "audio_mute" => 0x1008ff12,
        "audio_vol_down" => 0x1008ff11,
        "audio_vol_up" => 0x1008ff13,
        "audio_play" | "audio_pause" => 0x1008ff14,
        "audio_stop" => 0x1008ff15,
        "audio_prev" => 0x1008ff16,
        "audio_next" => 0x1008ff17,
        _ => return Err(err("Invalid key code specified.")),
    })
}
fn mapping(conn: &RustConnection) -> Result<xproto::GetKeyboardMappingReply> {
    let setup = conn.setup();
    conn.get_keyboard_mapping(setup.min_keycode, setup.max_keycode - setup.min_keycode + 1)
        .map_err(failure)?
        .reply()
        .map_err(failure)
}
fn lookup(
    conn: &RustConnection,
    map: &xproto::GetKeyboardMappingReply,
    symbol: u32,
) -> Option<(u8, usize)> {
    map.keysyms
        .chunks(map.keysyms_per_keycode as usize)
        .enumerate()
        .find_map(|(row, keys)| {
            keys.iter()
                .take(4)
                .position(|k| *k == symbol)
                .map(|column| (conn.setup().min_keycode + row as u8, column))
        })
}
fn mapped_event(
    conn: &RustConnection,
    map: &xproto::GetKeyboardMappingReply,
    symbol: u32,
    down: bool,
    flags: u32,
) -> Result<()> {
    let (code, column) = lookup(conn, map, symbol)
        .ok_or_else(|| err("Invalid key code specified for current X11 layout."))?;
    let implicit_shift = column % 2 == 1 && flags & 1 == 0;
    let shift = if implicit_shift {
        Some(
            lookup(conn, map, 0xffe1)
                .ok_or_else(|| err("No Shift key in X11 layout"))?
                .0,
        )
    } else {
        None
    };
    let altgr = if column >= 2 {
        Some(
            lookup(conn, map, 0xfe03)
                .ok_or_else(|| err("No AltGr key in X11 layout"))?
                .0,
        )
    } else {
        None
    };
    if down {
        for modifier in [shift, altgr].into_iter().flatten() {
            fake(conn, xproto::KEY_PRESS_EVENT, modifier)?;
        }
    }
    let mut result = fake(
        conn,
        if down {
            xproto::KEY_PRESS_EVENT
        } else {
            xproto::KEY_RELEASE_EVENT
        },
        code,
    );
    if !down || result.is_err() {
        for modifier in [altgr, shift].into_iter().flatten() {
            let release = fake(conn, xproto::KEY_RELEASE_EVENT, modifier);
            if result.is_ok() {
                result = release;
            }
        }
    }
    result
}
pub(crate) fn key_event(symbol: u32, down: bool, flags: u32) -> Result<()> {
    let (conn, _) = connect()?;
    let map = mapping(&conn)?;
    mapped_event(&conn, &map, symbol, down, flags)
}
pub(crate) fn unicode(character: char) -> Result<()> {
    let (conn, _) = connect()?;
    let map = mapping(&conn)?;
    let symbol = match character {
        '\n' | '\r' => 0xff0d,
        '\t' => 0xff09,
        c if (c as u32) <= 0xff => c as u32,
        c => 0x01000000 | c as u32,
    };
    if lookup(&conn, &map, symbol).is_some() {
        mapped_event(&conn, &map, symbol, true, 0)?;
        return mapped_event(&conn, &map, symbol, false, 0);
    }
    // Reserve an unused mapping, restoring it even if injection fails. A populated
    // key is never overwritten. MappingNotify lets clients refresh their key maps.
    let width = map.keysyms_per_keycode as usize;
    let (index, original) = map
        .keysyms
        .chunks(width)
        .enumerate()
        .find(|(_, row)| row.iter().all(|k| *k == 0))
        .ok_or_else(|| err("No unused X11 keycode is available for Unicode input"))?;
    let code = conn.setup().min_keycode + index as u8;
    let mut symbols = vec![0; width];
    symbols[0] = symbol;
    conn.change_keyboard_mapping(1, code, width as u8, &symbols)
        .map_err(failure)?
        .check()
        .map_err(failure)?;
    struct Restore<'a> {
        conn: &'a RustConnection,
        code: u8,
        original: &'a [u32],
    }
    impl Drop for Restore<'_> {
        fn drop(&mut self) {
            if let Ok(cookie) = self.conn.change_keyboard_mapping(
                1,
                self.code,
                self.original.len() as u8,
                self.original,
            ) {
                let _ = cookie.check();
            }
        }
    }
    let _restore = Restore {
        conn: &conn,
        code,
        original,
    };
    std::thread::sleep(std::time::Duration::from_millis(10));
    fake(&conn, xproto::KEY_PRESS_EVENT, code)?;
    let result = fake(&conn, xproto::KEY_RELEASE_EVENT, code);
    std::thread::sleep(std::time::Duration::from_millis(10));
    result
}
pub(crate) fn displays() -> Result<Vec<Display>> {
    let (conn, screen) = connect()?;
    let root = &conn.setup().roots[screen];
    if let Ok(cookie) = conn.randr_get_monitors(root.root, true)
        && let Ok(reply) = cookie.reply()
    {
        let mut displays: Vec<_> = reply
            .monitors
            .into_iter()
            .map(|m| Display {
                id: m.name as u64,
                x: m.x as i64,
                y: m.y as i64,
                width: m.width as usize,
                height: m.height as usize,
                is_main: m.primary,
            })
            .collect();
        if !displays.is_empty() {
            if !displays.iter().any(|d| d.is_main) {
                displays[0].is_main = true;
            }
            return Ok(displays);
        }
    }
    Ok(vec![Display {
        id: screen as u64,
        x: 0,
        y: 0,
        width: root.width_in_pixels as usize,
        height: root.height_in_pixels as usize,
        is_main: true,
    }])
}
pub(crate) fn capture(_display: &Display, rect: Rect) -> Result<Bitmap> {
    let (conn, screen) = connect()?;
    let setup = conn.setup();
    let root = &setup.roots[screen];
    let x = i16::try_from(rect.x).map_err(|_| err("X11 capture origin overflow"))?;
    let y = i16::try_from(rect.y).map_err(|_| err("X11 capture origin overflow"))?;
    let width = u16::try_from(rect.width).map_err(|_| err("X11 capture width overflow"))?;
    let height = u16::try_from(rect.height).map_err(|_| err("X11 capture height overflow"))?;
    let reply = conn
        .get_image(
            ImageFormat::Z_PIXMAP,
            root.root,
            x,
            y,
            width,
            height,
            u32::MAX,
        )
        .map_err(failure)?
        .reply()
        .map_err(failure)?;
    let format = setup
        .pixmap_formats
        .iter()
        .find(|f| f.depth == reply.depth)
        .ok_or_else(|| err("Unknown X11 pixel format"))?;
    let visual = root
        .allowed_depths
        .iter()
        .flat_map(|d| &d.visuals)
        .find(|v| v.visual_id == reply.visual)
        .ok_or_else(|| err("Unknown X11 visual"))?;
    if !matches!(format.bits_per_pixel, 16 | 24 | 32) || format.scanline_pad == 0 {
        return Err(err("Unsupported X11 pixel format"));
    }
    let bpp = format.bits_per_pixel as usize / 8;
    let pad = format.scanline_pad as usize;
    let stride = (rect.width * format.bits_per_pixel as usize).div_ceil(pad) * pad / 8;
    if reply.data.len() < stride * rect.height {
        return Err(err("Truncated X11 image"));
    }
    fn channel(pixel: u32, mask: u32) -> u8 {
        if mask == 0 {
            return 0;
        }
        let value = (pixel & mask) >> mask.trailing_zeros();
        let max = mask >> mask.trailing_zeros();
        (value as u64 * 255 / max as u64) as u8
    }
    let mut bitmap = Bitmap::empty(rect.width, rect.height)?;
    for y in 0..rect.height {
        for x in 0..rect.width {
            let source = &reply.data[y * stride + x * bpp..][..bpp];
            let mut pixel = 0u32;
            for (index, byte) in source.iter().enumerate() {
                let shift = if setup.image_byte_order == ImageOrder::LSB_FIRST {
                    index
                } else {
                    bpp - index - 1
                };
                pixel |= (*byte as u32) << (8 * shift);
            }
            let dest = y * bitmap.byte_width + x * 4;
            bitmap.image[dest..dest + 4].copy_from_slice(&[
                channel(pixel, visual.blue_mask),
                channel(pixel, visual.green_mask),
                channel(pixel, visual.red_mask),
                255,
            ]);
        }
    }
    Ok(bitmap)
}
