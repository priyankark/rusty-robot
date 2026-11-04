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

#[cfg(test)]
mod tests {
    use super::*;
    type Contents = Vec<Vec<(String, Vec<u8>)>>;
    fn contents(c: &SystemClipboard) -> Contents {
        let mut result = Vec::new();
        for item in c.snapshot().unwrap().iter() {
            let mut formats = Vec::new();
            unsafe {
                let types: Retained<NSArray<NSString>> = msg_send![&item, types];
                for format in types.iter() {
                    let data: Retained<NSData> = msg_send![&item, dataForType: &*format];
                    formats.push((format.to_string(), data.to_vec()));
                }
            }
            formats.sort();
            result.push(formats);
        }
        result
    }
    #[test]
    fn private_pasteboard_restores_all_items_and_respects_replacement() {
        // A uniquely named board never reads or modifies the user's clipboard.
        // Pasteboard services work in hosted macOS CI without input/capture grants.
        unsafe {
            let class = AnyClass::get(c"NSPasteboard").unwrap();
            let board: Retained<AnyObject> = msg_send![class, pasteboardWithUniqueName];
            let mut c = SystemClipboard { board };
            let item_class = AnyClass::get(c"NSPasteboardItem").unwrap();
            let mut items = Vec::new();
            for i in 0..2 {
                let item: Retained<AnyObject> = msg_send![item_class, new];
                for (format, value) in [
                    ("public.utf8-plain-text", format!("original {i} 中文")),
                    ("public.html", format!("<b>original {i}</b>")),
                ] {
                    let data = NSData::with_bytes(value.as_bytes());
                    let ok: bool =
                        msg_send![&item, setData: &*data, forType: &*NSString::from_str(format)];
                    assert!(ok);
                }
                items.push(item);
            }
            c.write_items(&NSArray::from_retained_slice(&items))
                .unwrap();
            let original = contents(&c);
            assert_eq!(original.len(), 2);
            assert!(original.iter().all(|item| item.len() == 2));
            let text = "literal\n    café 中文 🚀\n".repeat(256);
            let lease = c.install_text(&text).unwrap();
            assert_eq!(contents(&c)[0][0].1, text.as_bytes());
            assert!(c.restore_if_owned(lease).unwrap());
            assert_eq!(contents(&c), original);

            let lease = c.install_text("first owner").unwrap();
            let _: isize = msg_send![&c.board, clearContents];
            let ok: bool = msg_send![&c.board, setString: &*NSString::from_str("another owner"), forType: &*NSString::from_str("public.utf8-plain-text")];
            assert!(ok);
            assert!(!c.restore_if_owned(lease).unwrap());
            assert_eq!(contents(&c)[0][0].1, b"another owner");
            let _: () = msg_send![&c.board, releaseGlobally];
        }
    }
}
