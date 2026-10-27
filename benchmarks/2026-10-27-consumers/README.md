# Real consumer comparison — 27 October 2026

This snapshot separates three questions:

1. Actual app video delivery: capture, resize, hash, JPEG encoding, authenticated
   WebSocket transport and receipt. AirCodum runs inside a real isolated VS Code
   extension host. Agentum runs its real VNC server, without capture mocks.
2. Verified native input: socket commands through the selected native backend to
   the actual VS Code document or owned AppKit controls. Video is stopped for
   AirCodum input. Agentum's unrelated capture subscription is stubbed for its
   input-only cases; its stream benchmark has no capture/transport mocks.
3. Encoded-image processing: identical synthetic PNG bytes through Jimp or the
   composed Rust decode/resize/hash/JPEG pipeline. This does not time RobotJS
   itself; RobotJS does not implement the apps' image encoding.

## Controlled conditions

One Apple M6 Mac; three independent processes per backend, sequential runs,
backend order alternating by round. Both live-video paths request 1440×810,
JPEG quality 85 and 30 FPS with adaptive settings disabled. Ten measured seconds
per static/motion workload, after capture warm-up. An owned full-screen AppKit
window renders static content or a 30 Hz changing grid. Neither desktop captures
nor credentials are saved in the public results.

The original video path is the apps' existing macOS `screencapture` + `sips`
pipeline. The Rust path captures through ScreenCaptureKit, converts pixels,
resizes using SIMD and a shared four-thread pool, hashes with BLAKE3 and encodes
JPEG. Rust admits at most two outstanding media tasks; each app schedules one
live capture at a time. Both paths retain periodic refresh and transport
backpressure. RobotJS input comes from the actual app dependency, not a JS mock.

Video records every received frame's timestamp, byte count, dimensions and
encoded SHA-256. The first and last JPEGs are decoded after timing. Motion must
produce multiple distinct frames. Static FPS is deliberately low because of
unchanged-frame suppression; it is not a throughput score. Initial privacy/UI
transitions can generate additional static frames.

Input uses one warm-up and five measured repetitions per case per process,
rotates case order, and verifies actual text/button state. Setup/focus/reset are
outside timing. Observation polls every 2 ms with a 2-second failure timeout.
Incorrect output has null delivered latency and is never assigned a speedup.
The original path corrupts the supplementary-plane emoji in these fixtures;
VS Code also drops the tested line break. Failed trials remain in the raw files.

Image processing uses one warm-up and 15 samples per case/process, at JPEG
quality 85. Decoded dimensions are checked. PSNR uses an independent area-average
RGB reference generated from the original synthetic pixels; it includes
resampler and JPEG differences. Quality numbers alone are not equal-fidelity
proof. JPEG bytes and PSNR accompany speed results.

## Limits

- Local macOS/loopback results, not internet throughput or remote-phone rendering.
- First-frame latency ends at receipt of JPEG bytes. Ping RTT measures control
  responsiveness while streaming, not glass-to-glass latency.
- CPU includes only the current Node/extension-host process (and Rust workers).
  It excludes original capture/resize subprocesses, WindowServer, VS Code's
  renderer and the stimulus. It cannot establish total CPU/energy savings.
- Three process-level runs expose repeatability but are not a broad hardware
  study. The report uses median run FPS and displays the range, without claiming
  statistical significance from correlated frame samples.
- Rust's one-shot ScreenCaptureKit path still falls below the requested 30 FPS.
  Persistent capture streams, dirty-region transport and remote-client rendering
  tests remain follow-up work before calling this a finished VNC system.
- Original macOS capture encodes before resize; Rust captures raw and encodes
  once. This compares real app pipelines, not identical capture algorithms.

## Evidence and reproduction

`manifest.json` identifies source revisions, addon SHA-256, tools and display.
`aircodum.json`, `agentum-stream.json` and `agentum-input.json` contain final runs.
`agentum-before-batching.json` preserves the slower per-character checkpoint;
its numbers are not mixed into the final comparison. `processing/raw.json`
contains the isolated image workload. `summary.json` is generated from these.

Build the pinned addon and TypeScript in each consumer as documented in its
`RUSTY_ROBOT.md`. On an authorized, unlocked Mac with a 1920×1080 primary display:

```sh
# AirCodum/extension: actual isolated VS Code extension host
caffeinate -d -i node scripts/benchmark-input.cjs
# Agentum: run sequentially, never concurrently with desktop tests
caffeinate -d -i node scripts/benchmark-stream.cjs
caffeinate -d -i node scripts/benchmark-input.cjs
# rusty-robot: explicit installed Jimp dependency from the consumer
BENCHMARK_JIMP=/absolute/consumer/node_modules/jimp node scripts/benchmark-processing.cjs
python3 scripts/consumer-report.py
python3 scripts/consumer-report.py --check
```

The shared stream stimulus/measurement code and processing harness are in
`benchmarks/consumer-harness`. Consumer launchers are committed in their PRs.
