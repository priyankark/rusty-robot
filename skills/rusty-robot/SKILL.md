---
name: rusty-robot
description: Integrate Rusty Robot desktop automation in Rust applications or migrate RobotJS input and image workflows. Use for native mouse/keyboard input, screen capture, bitmap search, and consumer verification with this library.
---

# Rusty Robot

Use the `rusty-robot-desktop` Cargo package (`rusty_robot` import) from
https://github.com/priyankark/rusty-robot. The Node package is `@priyankark/rusty-robot`. This is a library, not a CLI or MCP
server. Inspect the consumer before selecting an
integration boundary; keep its authentication, focus policy, and input ownership.

## Working sequence

1. Identify OS, interactive session, crate revision, and the consumer's current
   input/capture path. Start with permission queries, display enumeration, and
   cursor position. Confirm the session is unlocked and the owned target has focus
   before input; an available display alone does not establish either.
   `Option<bool>::None` means the platform has no corresponding
   permission API, not that access is granted.
2. For Rust, read [Rust API recipes](references/rust-api.md). For Node/TypeScript,
   also read [consumer integration](references/node-integration.md). Pin the published package version; use a release commit for source builds.
3. Compile the smallest real consumer path. Use an owned test window for live
   input. Assert what the target app received, not merely a successful API return.
   Reuse existing task authorization; this skill adds no permission to message,
   publish, or operate unrelated applications.
4. Add regressions at the boundary that failed: Rust contracts, adapter forwarding,
   input ownership, event ordering, or native delivery. Record executed tests,
   platform, display scale, skipped cases, and meaningful limitations separately.

## Non-obvious contracts

- `Robot` is a stateless unit struct. Delays and held-input state are process-wide.
  Serialize an automation sequence in the consumer; individual API locks do not
  make a multi-call sequence atomic. Release held keys/buttons during cancellation.
- Desktop positions are signed logical coordinates. Bitmap positions are unsigned
  physical pixels. Preserve `screen_x`, `screen_y`, `scale_x`, `scale_y`; use bitmap
  conversion methods for image-based clicks. A capture rectangle must fit inside
  one display. `capture(None)` selects the main display, not the entire desktop.
- `get_pixel_color` accepts nonnegative main-display coordinates. For a monitor
  at a negative origin, use `capture(Some(rect))` and bitmap `color_at`.
- A key is named `enter`, not `return`; modifiers include `command`, `control`,
  `alt`, and `shift`. Use `type_string` for Unicode text, `key_tap` for shortcuts.
  macOS `type_string` batches up to 20 UTF-16 units per native event with a 10 ms minimum interval per batch. Newlines use Return events. `type_string_delayed` retains per-scalar pacing.
- `drag_mouse` moves with a held button; callers still press/release it. When
  adapting a stream of mouse events, track the held button and use drag movement.
- Bitmap storage is BGR/BGRA with row stride; it is not tightly packed RGBA.
  Search ignores alpha. Pixel strings are six-digit RGB without `#`.
- OS differences matter: macOS 14+ needs Accessibility and Screen Recording;
  Linux needs X11/XTest and `DISPLAY` (no native Wayland); Windows needs an
  interactive desktop. Do not treat cross-compilation as native verification.
- Measure consumer workloads before replacing capture: recorded macOS one-pixel
  and small-region captures were slower than RobotJS. Avoid per-pixel captures;
  capture once and search the bitmap when appropriate.

For library changes, run `cargo test --all-features --locked`, the corresponding
`--no-default-features` run, fmt and Clippy. Native test instructions and benchmark
limitations live in the repository's `docs/testing.md` and `docs/benchmarks.md`.

## Native media for Node hosts

Use `crates/rusty-robot-media` for Rust capture/resize/hash/encode and the shared
`crates/rusty-robot-node` bridge for Node apps. Install `@priyankark/rusty-robot@0.1.2` for verified
prebuilt binaries, or build from source with `node scripts/build-node.cjs`.
`await addon.captureFrame(1440, 85, previousFingerprint, forceRefresh, 'jpeg')`
returns dimensions, fingerprint and optional `data`. Missing data means unchanged;
force a periodic refresh for recovering subscribers. `resizeImage` accepts JPEG/PNG.
Pixel work runs on workers with two outstanding jobs maximum and four resize threads.
Keep one live capture in flight, drop obsolete results after stop/settings changes,
and handle queue rejection. Do not silently fall back to JS when Rust is selected.
Benchmark real socket delivery with fixed dimensions/quality/FPS; distinguish host
CPU from subprocess/system CPU and invalid output from successful delivery.
