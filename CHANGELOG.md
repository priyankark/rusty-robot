# Changelog

## 0.1.1 — 2026-10-27

- Make release checksums directly verifiable alongside downloaded archives.
- Bound Linux desktop CI package installation and use the validated Ubuntu runner.
- Keep core, media, and Node bridge versions aligned for consumer releases.
- Runtime behavior and the published 0.1.0 benchmark measurements are unchanged.

## 0.1.0 — 2026-10-27

Initial public preview of the Rust RobotJS port.

- Mouse, keyboard, Unicode, display enumeration, capture, and permission APIs.
- Rust bitmap search and BMP/optional PNG codecs with signed/DPI coordinates.
- macOS, Linux/X11, and Windows backends; platform validation is tracked separately.
- 77 mapped upstream cases, exhaustive regressions, and opt-in native GUI fixtures.
- Public 21-operation benchmark data, report, and reproducible runner.
- Agent skill, contributor guide, CI, documentation deployment, and release packaging.
- Composable Rust media and Node-API crates: bounded asynchronous capture/resize/hash/encode.
- Batched macOS Unicode insertion, preserving scalar boundaries and native Return events.
- Real AirCodum/Agentum input and streaming comparison with raw evidence.

Known limits: native Wayland unsupported; macOS small captures are slower than
RobotJS; Unicode typing has a macOS pacing floor; pre-1.0 APIs may change.
