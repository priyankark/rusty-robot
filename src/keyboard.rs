use crate::{Result, Robot, error::err, platform};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};
static DELAY: AtomicU64 = AtomicU64::new(10);
fn wait() {
    thread::sleep(Duration::from_millis(DELAY.load(Ordering::Relaxed)));
}
fn text_interval(delay: Duration) -> Duration {
    // Quartz delivery is asynchronous. Chromium's text services corrupted rapid
    // Unicode input even when every keydown arrived. Leave time to commit text;
    // this floor also applies when callers request zero delay or very high CPM.
    #[cfg(target_os = "macos")]
    let delay = delay.max(Duration::from_millis(10));
    delay
}
fn wait_for_text(delay: Duration) {
    thread::sleep(text_interval(delay));
}

// Private transport boundary: production uses native events; fault-injection
// tests can exercise cleanup without touching the real desktop.
trait KeyboardOutput {
    fn key_code(&mut self, key: &str) -> Result<u32>;
    fn key_event(&mut self, code: u32, down: bool, flags: u32) -> Result<()>;
    fn unicode(&mut self, character: char) -> Result<()>;
}
struct NativeKeyboard;
impl KeyboardOutput for NativeKeyboard {
    fn key_code(&mut self, key: &str) -> Result<u32> {
        platform::key_code(key)
    }
    fn key_event(&mut self, code: u32, down: bool, flags: u32) -> Result<()> {
        platform::key_event(code, down, flags)
    }
    fn unicode(&mut self, character: char) -> Result<()> {
        platform::unicode(character)
    }
}
fn type_character(output: &mut impl KeyboardOutput, character: char) -> Result<()> {
    // Text controls do not consistently interpret a Unicode LF/CR event as
    // Return (Quartz drops LF in Chromium). Send the native Return key instead.
    // Callers already hold the input lock; do not recurse through key_tap.
    if matches!(character, '\n' | '\r') {
        let code = output.key_code("enter")?;
        toggle(output, code, true, 0)?;
        toggle(output, code, false, 0)
    } else {
        output.unicode(character)
    }
}
fn modifiers(names: &[&str]) -> Result<u32> {
    let mut bits = 0;
    for name in names {
        bits |= match *name {
            "none" => 0,
            "shift" | "right_shift" => 1,
            "control" | "left_control" | "right_control" => 2,
            "alt" | "right_alt" => 4,
            "command" => 8,
            _ => return Err(err("Invalid key flag specified.")),
        };
    }
    Ok(bits)
}
fn toggle(output: &mut impl KeyboardOutput, code: u32, down: bool, flags: u32) -> Result<()> {
    let mut active = if down { 0 } else { flags };
    // Resolve the full chord before emitting anything. A later lookup error
    // must not strand an earlier modifier in the pressed state.
    let modifier_keys = [(8, "command"), (4, "alt"), (2, "control"), (1, "shift")]
        .into_iter()
        .filter(|(bit, _)| flags & bit != 0)
        .map(|(bit, name)| output.key_code(name).map(|code| (bit, code)))
        .collect::<Result<Vec<_>>>()?;
    if down {
        let mut pressed: Vec<(u32, u32)> = Vec::new();
        for (bit, modifier) in modifier_keys {
            let next = active | bit;
            if let Err(error) = output.key_event(modifier, true, next) {
                for (bit, code) in pressed.into_iter().rev() {
                    active &= !bit;
                    let _ = output.key_event(code, false, active);
                }
                return Err(error);
            }
            active = next;
            pressed.push((bit, modifier));
        }
        if let Err(error) = output.key_event(code, true, active) {
            for (bit, code) in pressed.into_iter().rev() {
                active &= !bit;
                let _ = output.key_event(code, false, active);
            }
            return Err(error);
        }
    } else {
        let mut result = output.key_event(code, false, active);
        for (bit, code) in modifier_keys.into_iter().rev() {
            active &= !bit;
            let release = output.key_event(code, false, active);
            if result.is_ok() {
                result = release;
            }
        }
        result?;
    }
    Ok(())
}
impl Robot {
    /// Set the delay after keyboard events. On macOS, Unicode text retains a
    /// 10 ms minimum delivery interval; physical key events use the given delay.
    pub fn set_keyboard_delay(&self, ms: u64) -> Result<()> {
        DELAY.store(ms, Ordering::Relaxed);
        Ok(())
    }
    pub fn key_tap(&self, key: &str, names: &[&str]) -> Result<()> {
        let flags = modifiers(names)?;
        let code = platform::key_code(key)?;
        let _lock = platform::input_lock()?;
        toggle(&mut NativeKeyboard, code, true, flags)?;
        let result = toggle(&mut NativeKeyboard, code, false, flags);
        wait();
        result
    }
    pub fn key_toggle(&self, key: &str, state: &str, names: &[&str]) -> Result<()> {
        let down = platform::down(state)?;
        let flags = modifiers(names)?;
        let code = platform::key_code(key)?;
        let _lock = platform::input_lock()?;
        toggle(&mut NativeKeyboard, code, down, flags)?;
        wait();
        Ok(())
    }
    pub fn unicode_tap(&self, character: char) -> Result<()> {
        let _lock = platform::input_lock()?;
        type_character(&mut NativeKeyboard, character)?;
        wait_for_text(Duration::from_millis(DELAY.load(Ordering::Relaxed)));
        Ok(())
    }
    pub fn type_string(&self, text: &str) -> Result<()> {
        let _lock = platform::input_lock()?;
        for character in text.chars() {
            type_character(&mut NativeKeyboard, character)?;
            wait_for_text(Duration::from_millis(DELAY.load(Ordering::Relaxed)));
        }
        Ok(())
    }
    /// Type at the requested characters per minute, subject to OS delivery
    /// overhead and macOS's 10 ms minimum Unicode interval.
    pub fn type_string_delayed(&self, text: &str, cpm: u64) -> Result<()> {
        if cpm == 0 {
            return Err(err("Typing speed must be greater than zero"));
        }
        let delay = Duration::from_secs_f64(60.0 / cpm as f64);
        let _lock = platform::input_lock()?;
        for character in text.chars() {
            type_character(&mut NativeKeyboard, character)?;
            wait_for_text(delay);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "keyboard/tests.rs"]
mod tests;
