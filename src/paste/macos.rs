//! Native multi-item, multi-format snapshot. No subprocesses or text-only fallback.
use super::PasteClipboard;
use crate::{Result, error::err};
use objc2::{
    msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject},
};
use objc2_foundation::{NSArray, NSData, NSString};
#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}

/// macOS clipboard adapter. Preserves all readable data representations/items,
/// rejects promised/unreadable formats before mutation, and checks changeCount
/// before restoration. NSPasteboard has no atomic compare-and-swap: hosts must
/// serialize clipboard writers; an external change can race the final check.
pub struct SystemClipboard {
    board: Retained<AnyObject>,
}
pub struct ClipboardLease {
    items: Retained<NSArray<AnyObject>>,
    revision: isize,
}
impl SystemClipboard {
    pub fn new() -> Result<Self> {
        unsafe {
            let class = AnyClass::get(c"NSPasteboard")
                .ok_or_else(|| err("AppKit clipboard unavailable"))?;
            let board: Retained<AnyObject> = msg_send![class, generalPasteboard];
            Ok(Self { board })
        }
    }
    fn revision(&self) -> isize {
        unsafe { msg_send![&self.board, changeCount] }
    }
    fn snapshot(&self) -> Result<Retained<NSArray<AnyObject>>> {
        unsafe {
            let source: Option<Retained<NSArray<AnyObject>>> =
                msg_send![&self.board, pasteboardItems];
            let class = AnyClass::get(c"NSPasteboardItem")
                .ok_or_else(|| err("Pasteboard items unavailable"))?;
            let mut items = Vec::new();
            if let Some(source) = source {
                for item in source.iter() {
                    let types: Retained<NSArray<NSString>> = msg_send![&item, types];
                    let copy: Retained<AnyObject> = msg_send![class, new];
                    for format in types.iter() {
                        let data: Option<Retained<NSData>> = msg_send![&item,dataForType:&*format];
                        let data = data.ok_or_else(|| {
                            err("Clipboard has an unreadable format; preservation refused")
                        })?;
                        let ok: bool = msg_send![&copy, setData: &*data, forType: &*format];
                        if !ok {
                            return Err(err("Could not snapshot clipboard format"));
                        }
                    }
                    items.push(copy);
                }
            }
            Ok(NSArray::from_retained_slice(&items))
        }
    }
    fn write_items(&self, items: &NSArray<AnyObject>) -> Result<()> {
        unsafe {
            let _: isize = msg_send![&self.board, clearContents];
            if items.is_empty() {
                return Ok(());
            }
            let ok: bool = msg_send![&self.board,writeObjects:items];
            if ok {
                Ok(())
            } else {
                Err(err("Clipboard restoration failed"))
            }
        }
    }
}
impl PasteClipboard for SystemClipboard {
    type Lease = ClipboardLease;
    fn install_text(&mut self, text: &str) -> Result<ClipboardLease> {
        let revision = self.revision();
        let items = self.snapshot()?;
        if self.revision() != revision {
            return Err(err("Clipboard changed while snapshotting"));
        }
        unsafe {
            let _: isize = msg_send![&self.board, clearContents];
            let ok: bool = msg_send![&self.board, setString: &*NSString::from_str(text), forType: &*NSString::from_str("public.utf8-plain-text")];
            if !ok {
                let restore = self.write_items(&items);
                return Err(err(format!(
                    "Clipboard text installation failed; rollback: {restore:?}"
                )));
            }
        }
        Ok(ClipboardLease {
            items,
            revision: self.revision(),
        })
    }
    fn restore_if_owned(&mut self, lease: ClipboardLease) -> Result<bool> {
        if self.revision() != lease.revision {
            return Ok(false);
        }
        self.write_items(&lease.items)?;
        Ok(true)
    }
}
