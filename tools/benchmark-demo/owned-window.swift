import Foundation
import AppKit
import CoreGraphics
let args = CommandLine.arguments
let pid = Int(args[1])!
let target = CGRect(x:Double(args[2])!, y:Double(args[3])!, width:Double(args[4])!, height:Double(args[5])!)
if args.contains("--activate") {
    guard NSRunningApplication(processIdentifier:Int32(pid))?.activate(options:[.activateAllWindows]) == true else { exit(1) }
    Thread.sleep(forTimeInterval:0.4)
}
let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly,.excludeDesktopElements], kCGNullWindowID) as! [[String:Any]]
for window in windows {
    guard let raw = window[kCGWindowBounds as String] as? NSDictionary,
          let bounds = CGRect(dictionaryRepresentation:raw as CFDictionary),
          bounds.intersects(target), (window[kCGWindowLayer as String] as? Int) == 0,
          (window[kCGWindowAlpha as String] as? Double ?? 0) > 0 else { continue }
    let owner = window[kCGWindowOwnerPID as String] as? Int ?? -1
    if owner == pid {
        if bounds.contains(target) { print("owned-window-visible"); exit(0) }
        continue
    }
    // The capture rectangle must belong to the topmost ordinary app window.
    fputs("Owned capture rectangle is occluded or its window moved.\n",stderr)
    exit(1)
}
fputs("Owned window is not visible.\n",stderr)
exit(1)
