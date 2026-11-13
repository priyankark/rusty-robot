# Rusty Robot for Node.js

Native Rust desktop input, screen capture, resize, BLAKE3 frame fingerprints, and
JPEG/PNG processing. Installation includes all supported Node-API binaries: no
Rust toolchain, Git checkout, install scripts, or runtime downloads are required.

Install the verified release archive (also available before the npm mirror):

```sh
npm install https://github.com/priyankark/rusty-robot/releases/download/v0.1.2/priyankark-rusty-robot-0.1.2.tgz
```

Save this as `capture.cjs`, then run `node capture.cjs`:

```js
const robot = require('@priyankark/rusty-robot');
async function main() {
  console.log(robot.getPermissions());
  const frame = await robot.captureFrame(1440, 85, null, true, 'jpeg');
  console.log({ width: frame.width, height: frame.height, bytes: frame.data.length });
}
main().catch(error => { console.error(error.message); process.exitCode = 1; });
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

## macOS capture isolation (unreleased)

On macOS 27 and newer, the package automatically runs capture and resize in a
private worker with its own byte-identical copy of the host's Node executable.
This avoids the reproduced ScreenCaptureKit connection collision between Node
apps sharing one executable path. Keep using `captureFrame()` and `resizeImage()`;
no application-side process wrapper or capture fallback is needed. Input stays
in the host process with its existing ordering and permission checks.

The worker is reused across frames and closes after five idle seconds. Both media
APIs share the existing two-job admission limit and one native pool of at most
four resize threads. Image bytes are copied for transport; decoding, capture,
resizing, hashing and encoding still run in Rust. Each admitted media request has
a 15-second deadline including startup and queueing. On timeout, the owned worker
is terminated and all its pending requests reject with `ERR_RUSTY_MEDIA_TIMEOUT`.
A later explicit call can start a fresh worker; failed work is never replayed.
Worker startup/IPC failures reject with `ERR_RUSTY_MEDIA_WORKER`; native errors
retain their message and code. A native capture failure also discards its worker
and rejects pending requests, so stale ScreenCaptureKit callbacks cannot leak
into the next explicit request. Invalid arguments and queue saturation still throw
synchronously. The package verifies the native binary again inside the worker.

Runtime copies use copy-on-write when supported, are private to the user, and
are removed on normal exit, worker failure, idle shutdown or timeout. A separate
watchdog can terminate a worker even when a native call blocks its event loop,
including after abrupt host death. OS termination during runtime preparation can
leave a temporary directory, since no worker exists yet to clean it up.
No TCC grants, signing identities or other applications are changed. If the host
executable cannot run from a private copy (for example, a bundled runtime with
relative framework dependencies), startup fails explicitly; standalone Node is
the verified host. Older macOS, Linux and Windows retain their existing execution
path. Direct `.node` imports and the standalone Rust crate bypass this Node
package transport and do not acquire this isolation.
