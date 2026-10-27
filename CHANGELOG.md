# Changelog

## 0.1.0 — 2026-10-27

Initial public preview of the Rust RobotJS port.

- Mouse, keyboard, Unicode, display enumeration, capture, and permission APIs.
- Rust bitmap search and BMP/optional PNG codecs with signed/DPI coordinates.
- macOS, Linux/X11, and Windows backends; platform validation is tracked separately.
- 77 mapped upstream cases, exhaustive regressions, and opt-in native GUI fixtures.
- Public 21-operation benchmark data, report, and reproducible runner.
- Agent skill, contributor guide, CI, documentation deployment, and release packaging.

Known limits: native Wayland unsupported; macOS small captures are slower than
RobotJS; Unicode typing has a macOS pacing floor; pre-1.0 APIs may change.
