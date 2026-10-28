// Benchmark target: stdin resets fixtures outside timing and queries observed state; measured input comes through the OS.
import AppKit
import Foundation

func emit(_ value: [String: Any]) {
    let data = try! JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
    FileHandle.standardOutput.write(data + Data([10]))
}

final class Patch: NSView {
    var green = false
    var rgb: [Int]? = nil
    var checkerboard = false
    override var isFlipped: Bool { true }
    override func draw(_ dirtyRect: NSRect) {
        if checkerboard {
            NSGraphicsContext.current?.cgContext.setShouldAntialias(false)
            for y in 0..<Int(bounds.height) { for x in 0..<Int(bounds.width) {
                ((x+y)%2 == 0 ? NSColor.white : NSColor.black).setFill()
                NSRect(x:x,y:y,width:1,height:1).fill()
            }}
            return
        }
        (rgb.map { NSColor(srgbRed:Double($0[0])/255,green:Double($0[1])/255,blue:Double($0[2])/255,alpha:1) } ?? (green ? NSColor(srgbRed: 0, green: 1, blue: 0, alpha: 1) : NSColor(srgbRed: 1, green: 0, blue: 0, alpha: 1))).setFill()
        bounds.fill()
    }
}
final class Target: NSObject, NSApplicationDelegate {
    var window: NSWindow!
    let editor = NSTextView(frame: NSRect(x: 20, y: 20, width: 660, height: 350))
    let button = NSButton(title: "Native button", target: nil, action: nil)
    let status = NSTextField(labelWithString: "Rusty Robot · AppKit verification")
    var clicks = 0
    let patch = Patch(frame: NSRect(x: 400, y: 390, width: 200, height: 60))

    func applicationDidFinishLaunching(_ notification: Notification) {
        // AppKit routes standard Command shortcuts through the application's
        // Edit menu. Match a normal Cocoa editor's responder-chain actions.
        let menu = NSMenu()
        let editItem = NSMenuItem(title: "Edit", action: nil, keyEquivalent: "")
        let edit = NSMenu(title: "Edit")
        for (title, action, key) in [("Undo", "undo:", "z"), ("Redo", "redo:", "Z"),
                                     ("Select All", "selectAll:", "a")] {
            edit.addItem(NSMenuItem(title: title, action: NSSelectorFromString(action), keyEquivalent: key))
        }
        editItem.submenu = edit
        menu.addItem(editItem)
        NSApplication.shared.mainMenu = menu
        let screen = NSScreen.screens[0].frame
        window = NSWindow(contentRect: NSRect(x: 100, y: screen.maxY - 600, width: 700, height: 500),
                          styleMask: [.titled, .closable], backing: .buffered, defer: false)
        window.title = "Rusty Robot owned native test window"
        window.isReleasedWhenClosed = false
        window.level = .floating
        editor.font = NSFont.systemFont(ofSize: 18)
        editor.isRichText = false
        editor.allowsUndo = true
        editor.isAutomaticQuoteSubstitutionEnabled = false
        editor.isAutomaticDashSubstitutionEnabled = false
        editor.isAutomaticTextReplacementEnabled = false
        editor.isAutomaticSpellingCorrectionEnabled = false
        editor.autoresizingMask = [.width, .height]
        button.frame = NSRect(x: 20, y: 400, width: 180, height: 40)
        button.bezelStyle = .rounded
        button.target = self
        button.action = #selector(clicked)
        status.frame = NSRect(x: 20, y: 455, width: 660, height: 24)
        for view in [editor, button, status, patch] as [NSView] { window.contentView!.addSubview(view) }
        window.makeKeyAndOrderFront(nil)
        NSApplication.shared.activate(ignoringOtherApps: true)
        window.makeFirstResponder(editor)
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) {
            emit(["ready": true, "window": self.screenRect(self.window.contentView!),
                  "editor": self.screenRect(self.editor), "button": self.screenRect(self.button), "patch": self.screenRect(self.patch)])
        }
        DispatchQueue.global().async {
            while let line = readLine() {
                DispatchQueue.main.async {
                    guard let data = line.data(using: .utf8),
                          let command = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
                    else { return }
                    if let text = command["reset"] as? String {
                        self.editor.string = text
                        self.editor.setSelectedRange(NSRange(location: text.utf16.count, length: 0))
                        self.window.makeFirstResponder(self.editor)
                        self.editor.undoManager?.removeAllActions()
                    }
                    if let rgb = command["rgb"] as? [Int] { self.patch.rgb = rgb; self.patch.checkerboard = false; self.patch.needsDisplay = true; self.window.displayIfNeeded() }
                    if let pattern = command["checkerboard"] as? Bool { self.patch.checkerboard = pattern; self.patch.needsDisplay = true; self.window.displayIfNeeded() }
                    if let green = command["green"] as? Bool {
                        self.patch.green = green; self.patch.needsDisplay = true; self.window.displayIfNeeded()
                    }
                    if let phase = command["phase"] as? String { self.status.stringValue = phase }
                    emit(["text": self.editor.string, "clicks": self.clicks,
                          "active": NSApplication.shared.isActive,
                          "selectionLength": self.editor.selectedRange().length,
                          "cursorX": Int(NSEvent.mouseLocation.x),
                          "cursorY": Int(NSScreen.screens[0].frame.maxY - NSEvent.mouseLocation.y)])
                }
            }
            DispatchQueue.main.async { NSApplication.shared.terminate(nil) }
        }
    }
    func screenRect(_ view: NSView) -> [String: Int] {
        let rect = window.convertToScreen(view.convert(view.bounds, to: nil))
        return ["x": Int(rect.minX), "y": Int(NSScreen.screens[0].frame.maxY - rect.maxY),
                "width": Int(rect.width), "height": Int(rect.height)]
    }
    @objc func clicked() { clicks += 1; status.stringValue = "Native button clicks: \(clicks)" }
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
}

let app = NSApplication.shared
app.setActivationPolicy(.regular)
let target = Target()
app.delegate = target
app.run()
