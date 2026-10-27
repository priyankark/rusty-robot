# Node / TypeScript consumer integration

The shared `crates/rusty-robot-node` Node-API bridge composes input and the
`rusty-robot-media` capture/processing library. Build it with
`node scripts/build-node.cjs`; load `target/rusty-robot.node`. It is source-built
from Git, not a published npm package. Keep consumers pinned to a commit and
reuse this bridge instead of duplicating Rust wrappers.

```js
const native = require('/absolute/trusted/rusty-robot.node');
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
- Make the new backend explicitly selectable while it is being validated. A
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

For distribution, a source-build preview may require Rust and a native toolchain.
Do not advertise prebuilt binaries until each target is built, packaged, and tested.
Pin the Rust Git dependency to a commit and commit Cargo.lock. Document rollback
to the existing backend while the integration remains experimental.

## Dogfood example

[AirCodum Agentum PR #7](https://github.com/priyankark/agentum-cli/pull/7) is a
source-build example of this approach. It keeps RobotJS as the default and makes
Rust input and media selectable. Inspect its current review/CI status before adopting it;
a draft integration is not a published Node distribution. Its tests cover addon
load failures, drag-state transitions, authenticated VNC controls, and a separate
owned-window native workflow.

[AirCodum PR #33](https://github.com/priyankark/AirCodum/pull/33) uses the same shared
addon inside a real VS Code extension host. The public benchmark report includes
verified input, actual socket frame delivery and isolated processing. Read the
methodology before using its FPS or CPU figures as application-level guarantees.
