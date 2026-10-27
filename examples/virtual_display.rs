//! Test-only macOS virtual monitor. Holds a real OS display while running a command.
//! Uses private CoreGraphics APIs, as Chromium's virtual-display test helper does.
//! Never used by the library or ordinary tests. See docs/testing.md.
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("The virtual-display helper is only available on macOS");
    std::process::exit(1);
}
#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use objc2::{
        msg_send,
        rc::{Allocated, Retained},
        runtime::{AnyClass, AnyObject},
    };
    use objc2_foundation::{NSArray, NSPoint, NSSize, NSString};
    use std::{
        ffi::{CStr, c_void},
        ptr,
    };
    unsafe extern "C" {
        fn dispatch_get_global_queue(identifier: isize, flags: usize) -> *mut c_void;
    }
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGBeginDisplayConfiguration(config: *mut *mut c_void) -> i32;
        fn CGConfigureDisplayOrigin(config: *mut c_void, display: u32, x: i32, y: i32) -> i32;
        fn CGCompleteDisplayConfiguration(config: *mut c_void, option: u32) -> i32;
        fn CGCancelDisplayConfiguration(config: *mut c_void) -> i32;
    }
    fn class(name: &CStr) -> Result<&'static AnyClass, Box<dyn std::error::Error>> {
        AnyClass::get(name)
            .ok_or_else(|| format!("Private virtual-display class {name:?} is unavailable").into())
    }
    // SAFETY: These private signatures match the runtime interfaces used by Chromium
    // tests. All Objective-C objects are retained until the child process exits.
    let display: Retained<AnyObject> = unsafe {
        let descriptor: Retained<AnyObject> = msg_send![class(c"CGVirtualDisplayDescriptor")?, new];
        let _: () = msg_send![&descriptor,setName:&*NSString::from_str("rusty-robot test monitor")];
        let _: () =
            msg_send![&descriptor,setQueue:dispatch_get_global_queue(2,0).cast::<AnyObject>()];
        let _: () = msg_send![&descriptor,setMaxPixelsWide:1280u32];
        let _: () = msg_send![&descriptor,setMaxPixelsHigh:720u32];
        let _: () = msg_send![&descriptor,setSizeInMillimeters:NSSize::new(338.0,190.0)];
        let _: () = msg_send![&descriptor,setRedPrimary:NSPoint::new(0.64,0.33)];
        let _: () = msg_send![&descriptor,setGreenPrimary:NSPoint::new(0.30,0.60)];
        let _: () = msg_send![&descriptor,setBluePrimary:NSPoint::new(0.15,0.06)];
        let _: () = msg_send![&descriptor,setWhitePoint:NSPoint::new(0.3127,0.3290)];
        let _: () = msg_send![&descriptor,setVendorID:505u32];
        let _: () = msg_send![&descriptor,setProductID:1u32];
        let serial = std::process::id();
        let _: () = msg_send![&descriptor,setSerialNum:serial];
        let _: () = msg_send![&descriptor,setSerialNumber:serial];
        let allocated: Allocated<AnyObject> = msg_send![class(c"CGVirtualDisplay")?, alloc];
        let display: Option<Retained<AnyObject>> =
            msg_send![allocated,initWithDescriptor:&*descriptor];
        let display = display.ok_or("Could not create a virtual display")?;
        let allocated: Allocated<AnyObject> = msg_send![class(c"CGVirtualDisplayMode")?, alloc];
        let mode: Retained<AnyObject> =
            msg_send![allocated,initWithWidth:1280u32,height:720u32,refreshRate:60.0f64];
        let settings: Retained<AnyObject> = msg_send![class(c"CGVirtualDisplaySettings")?, new];
        let _: () = msg_send![&settings,setHiDPI:0u32];
        let _: () = msg_send![&settings,setModes:&*NSArray::from_slice(&[&*mode])];
        let applied: bool = msg_send![&display,applySettings:&*settings];
        if !applied {
            return Err("Could not apply virtual-display settings".into());
        }
        let id: u32 = msg_send![&display, displayID];
        let mut config = ptr::null_mut();
        if CGBeginDisplayConfiguration(&mut config) != 0 {
            return Err("Could not begin display configuration".into());
        }
        if CGConfigureDisplayOrigin(config, id, -1280, 0) != 0 {
            CGCancelDisplayConfiguration(config);
            return Err("Could not set negative display origin".into());
        }
        // Session-only configuration, never persisted to disk.
        if CGCompleteDisplayConfiguration(config, 0) != 0 {
            return Err("Could not apply display configuration".into());
        }
        eprintln!("Temporary display {id} at (-1280,0), 1280x720");
        display
    };
    std::thread::sleep(std::time::Duration::from_secs(2));
    let mut args = std::env::args_os().skip(1);
    let command = args
        .next()
        .ok_or("Usage: cargo run --example virtual_display -- COMMAND [ARGS...]")?;
    let result = std::process::Command::new(command).args(args).status();
    drop(display);
    std::thread::sleep(std::time::Duration::from_secs(1));
    let status = result?;
    if !status.success() {
        return Err(format!("Child command failed: {status}").into());
    }
    Ok(())
}
