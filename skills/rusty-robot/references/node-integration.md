# Node / TypeScript consumer integration

Install `@priyankark/rusty-robot@0.1.2`. The package includes SHA-256 verified
Node-API binaries for macOS arm64/x64, Linux GNU arm64/x64, and Windows arm64/x64/ia32.
It requires macOS 14+, glibc 2.35+ with X11 on Linux, or an interactive Windows desktop.
Node 18+ is supported, except Windows arm64 requires Node 22+. Native Wayland and
musl are not supported. No Rust compiler or install script is needed.

If npm reports that version as unpublished, install the same named package from
the public release archive:

```sh
npm install https://github.com/priyankark/rusty-robot/releases/download/v0.1.2/priyankark-rusty-robot-0.1.2.tgz
```

On macOS 27, two screen-capturing Node processes using the same executable path
were observed displacing each other's ScreenCaptureKit connection, even with
permission granted. Test multi-process consumers explicitly; a successful
permission query or single-process capture does not verify concurrent instances.
Do not terminate another user's capture session to make a check pass.

The shared `crates/rusty-robot-node` bridge composes input and the
`rusty-robot-media` capture/processing library. For development, build with
`node scripts/build-node.cjs` and explicitly load `target/rusty-robot.node`.

```js
const native = require('@priyankark/rusty-robot');
const frame = await native.captureFrame(1440, 85, previousHash, forceRefresh, 'jpeg');
if (frame.data) send(frame.data, { width: frame.width, height: frame.height });
previousHash = frame.fingerprint;
```

Media work runs off the JS loop. There are at most two outstanding jobs and four
shared resize threads. Keep one stream request in flight; discard completions
after cancellation/settings changes, and handle queue errors without silently
switching backends. Pixel conversion, resizing, hashing and encoding stay in Rust.

Before changing a real consumer:

- Inventory calls and existing tests. Preserve key aliases, modifier normalization,
  wheel units, display coordinate scaling, authenticated session gates, and drag
  ownership. Rust's `move_mouse` does not imply `drag_mouse`; track a held button
  in the adapter. Preserve release paths on disconnect/stop/cancel.
- Use Rust by default in the validated consumers; retain explicit original-backend opt-in. A
  configured Rust backend that fails to load must report failure, not silently
  fall back to a different native implementation.
- Pass native errors through. Do not accept NaN, infinity, fractional, or
  out-of-range coordinates via unchecked JS-to-Rust integer casts. Use bounded
  integer conversion before any input. Avoid shell execution to load/build paths.
- Select input and capture independently for controlled comparisons. The core
  bitmap API exposes BGR/BGRA; the media bridge handles conversion and returns
  JPEG/PNG bytes with explicit output dimensions.
- A synchronous binding keeps existing ordering but blocks the Node event loop
  during paced typing. Document this; a later worker/IPC architecture must retain
  ordering and release ownership rather than simply making each call parallel.

Verification should include (1) a fake addon test for adapter event order and
cancellation, (2) existing authenticated socket tests with the Rust adapter chosen,
(3) a built-addon query/error smoke test, and (4) real input to an owned fixture.
Report each layer separately. A mocked addon test does not prove OS delivery.
Keep native tests opt-in on developers' machines; use prepared desktop CI sessions.

For distribution, test the exact installed archive on every claimed native target.
Verify loading, media output, and corruption rejection with install scripts disabled.
Defer macOS Accessibility checks until an input action so users can grant permission
without breaking application startup. A failed native load must remain actionable.

## Dogfood examples

Agentum uses Rust input and media by default. Set `AGENTUM_INPUT_BACKEND=robotjs`
and `AGENTUM_MEDIA_BACKEND=robotjs` to explicitly select the original implementations.
AirCodum uses the corresponding `AIRCODUM_INPUT_BACKEND` and `AIRCODUM_MEDIA_BACKEND`
variables. Source builds can be selected with an absolute `*_RUSTY_ROBOT_PATH`.
Verify the installed app version: Agentum 2.1.0 and AirCodum 0.4.0 still used original
defaults; Agentum 2.2.0 and AirCodum 0.5.0 introduce packaged Rust defaults.

[AirCodum PR #33](https://github.com/priyankark/AirCodum/pull/33) uses the same shared
addon inside a real VS Code extension host. The public benchmark report includes
verified input, actual socket frame delivery and isolated processing. Read the
methodology before using its FPS or CPU figures as application-level guarantees.
