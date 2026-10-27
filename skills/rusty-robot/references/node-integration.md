# Node / TypeScript consumer integration

There is no first-party published Node package yet. For a synchronous RobotJS
consumer, a small Node-API binding is a practical initial seam: compile Rusty
Robot into a `.node` addon, map native `Result` errors to JS exceptions, and expose
only methods used by the consumer. Keep this separate from the core Rust crate.
A subprocess/IPC adapter is another option, but requires async ordering, lifetime,
and cancellation design; do not spawn a new process for every mouse event.

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
- Keep capture independent until its actual latency/format/scaling has been
  measured. The library exposes BGR/BGRA, while many JS encoders expect RGBA.
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
