# Roadmap

The public preview now has a composed Rust input/media backend, shared Node-API
bridge, agent skill, native desktop checks, and real AirCodum/Agentum benchmarks.
Both consumer integrations remain opt-in while their PRs are reviewed.

The next performance milestone is sustained 30 FPS with responsive input and
explicit fidelity budgets, followed by a reusable remote-display foundation:

1. Replace repeated one-shot macOS screenshots with a persistent ScreenCaptureKit
   stream. Bound buffering, discard stale frames, and verify stop/reconnect and
   display-change behavior before using it in the apps.
2. Compare resampling filters against desktop text and photographic fixtures.
   Publish throughput, fidelity and bytes together; avoid hiding blur behind
   an unchanged JPEG quality setting.
3. Measure total process-tree CPU and physical-device decode/render latency.
   Current reports measure host CPU and socket receipt, not glass-to-glass delay.
4. Add dirty-region transport, frame sequencing and latency-aware backpressure.
   Evaluate hardware encoding only with measured quality and interactive latency.
5. Establish signed/prebuilt Node binaries and registry releases after the
   consumer APIs settle. Continue Windows/Linux native media and multi-monitor,
   DPI, permission-loss and reconnect validation.
6. Evaluate Wayland portal integration with explicit compositor/permission
   support. X11 support does not imply Wayland support.

A future VNC application should compose these tested libraries rather than copy
capture or image-processing code into another JavaScript pipeline. Its transport,
authentication and remote rendering need their own verification. Performance
claims will continue to include slower cases and measurement limits.
