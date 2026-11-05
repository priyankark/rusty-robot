//! Literal paste with caller-owned clipboard storage and completion observation.
//! The host supplies a clipboard adapter so its format/ownership policy is explicit.
//! No arbitrary delay is treated as proof that the target consumed the clipboard.
use crate::{Result, Robot, error::err};
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::SystemClipboard;

/// A clipboard lease must snapshot supported formats before changing anything,
/// reject unsupported preservation, and restore only if it still owns the value.
/// Keep the adapter alive on X11 while serving/restoring the selection.
pub trait PasteClipboard {
    type Lease;
    fn install_text(&mut self, text: &str) -> Result<Self::Lease>;
    /// Return false if another owner replaced the clipboard; never overwrite it.
    fn restore_if_owned(&mut self, lease: Self::Lease) -> Result<bool>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasteOutcome {
    pub clipboard_restored: bool,
}
impl Robot {
    /// Paste literal text using a host clipboard adapter. `consumed` must wait
    /// until the target has read the data (or fail/cancel with bounded latency).
    /// Apps may transform pasted text; verify exact target contents when required.
    pub fn paste_text<C: PasteClipboard>(
        &self,
        clipboard: &mut C,
        text: &str,
        consumed: impl FnOnce() -> Result<()>,
    ) -> Result<PasteOutcome> {
        paste_with(clipboard, text, || {
            self.key_tap(
                "v",
                &[if cfg!(target_os = "macos") {
                    "command"
                } else {
                    "control"
                }],
            )?;
            consumed()
        })
    }
}
struct Restore<'a, C: PasteClipboard> {
    clipboard: &'a mut C,
    lease: Option<C::Lease>,
}
impl<C: PasteClipboard> Drop for Restore<'_, C> {
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take() {
            let _ = self.clipboard.restore_if_owned(lease);
        }
    }
}
fn paste_with<C: PasteClipboard>(
    clipboard: &mut C,
    text: &str,
    deliver: impl FnOnce() -> Result<()>,
) -> Result<PasteOutcome> {
    let lease = clipboard.install_text(text)?;
    let mut guard = Restore {
        clipboard,
        lease: Some(lease),
    };
    let sent = deliver();
    let restored = guard
        .clipboard
        .restore_if_owned(guard.lease.take().unwrap());
    match (sent, restored) {
        (Err(a), Err(b)) => Err(err(format!("{a}; clipboard restoration also failed: {b}"))),
        (Err(e), _) | (_, Err(e)) => Err(e),
        (Ok(()), Ok(value)) => Ok(PasteOutcome {
            clipboard_restored: value,
        }),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Clipboard {
        value: String,
        prior: String,
        installed: usize,
        restored: usize,
        replaced: bool,
        reject: bool,
    }
    impl PasteClipboard for Clipboard {
        type Lease = String;
        fn install_text(&mut self, t: &str) -> Result<String> {
            if self.reject {
                return Err(err("unsupported clipboard format"));
            }
            self.installed += 1;
            let old = std::mem::replace(&mut self.value, t.into());
            self.prior = old.clone();
            Ok(old)
        }
        fn restore_if_owned(&mut self, old: String) -> Result<bool> {
            self.restored += 1;
            if self.replaced {
                return Ok(false);
            }
            self.value = old;
            Ok(true)
        }
    }
    #[test]
    fn paste_restores_on_delivery_error_and_panic_and_respects_new_owner() {
        let mut c = Clipboard {
            value: "original".into(),
            ..Default::default()
        };
        assert!(paste_with(&mut c, "a\n    b", || Err(err("cancelled"))).is_err());
        assert_eq!(c.value, "original");
        assert_eq!(c.restored, 1);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            paste_with(&mut c, "literal", || panic!("abort"))
        }));
        assert_eq!(c.value, "original");
        assert_eq!(c.restored, 2);
        c.replaced = true;
        assert!(
            !paste_with(&mut c, "literal", || Ok(()))
                .unwrap()
                .clipboard_restored
        );
        c.reject = true;
        assert!(paste_with(&mut c, "literal", || panic!("must not post")).is_err());
        assert_eq!(c.installed, 3);
    }
}
