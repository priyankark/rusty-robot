# Rusty Robot for Node.js

Native Rust desktop input, screen capture, resize, BLAKE3 frame fingerprints, and
JPEG/PNG processing. Installation includes all supported Node-API binaries: no
Rust toolchain, Git checkout, install scripts, or runtime downloads are required.

```js
const robot = require('@priyankark/rusty-robot');
console.log(robot.getPermissions());
const frame = await robot.captureFrame(1440, 85, null, true, 'jpeg');
```

Supported: macOS 14+ (Apple Silicon and Intel), Windows 10/11 (x64, ARM64, and
32-bit Node hosts), and Linux with glibc 2.35+ and X11 (x64 and ARM64). Native
Windows ARM64 requires Node 22+; other targets support Node 18+. Native Wayland
and musl/Alpine are not supported. Accessibility/Screen Recording permissions are
required on macOS. Desktop APIs require an interactive desktop session.

The loader verifies the selected binary against the package's SHA-256 manifest.
Unsupported platforms and load failures produce an explicit error; they do not
silently switch implementations. Image processing runs on bounded workers with
at most two outstanding jobs and four shared resize threads.

Use one capture in flight; ignore obsolete results after cancellation. Unchanged
frames can omit `data`, so periodically request a forced refresh. Input calls are
synchronous; preserve ordering and release held keys/buttons during cancellation.

See the [source, API and benchmarks](https://github.com/priyankark/rusty-robot).
