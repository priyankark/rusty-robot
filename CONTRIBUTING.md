# Contributing

Start with an issue describing the observed behavior, OS, display scale, and a
minimal reproduction. Small fixes can go straight to a pull request. Do not
include screenshots of private desktops, credentials, or unrelated applications.

Use Rust 1.88+ and run:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings
cargo test --all-features --locked
cargo test --no-default-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
python3 scripts/check-benchmarks.py
```

Add a regression that observes the failure when changing behavior. Keep bitmap
and codec tests independent of desktop access. Native changes need an owned GUI
fixture on the affected platform: see [testing](docs/testing.md). Preserve the
RobotJS mapping and document deliberate API differences. Do not relabel ignored
desktop tests as passes. Benchmark changes need raw samples, machine/build
metadata, and clear distinction between synthetic and end-to-end workloads.

Public APIs return `Result`; validate dimensions, lengths, overflow, and native
return values before accessing buffers or emitting events. Keep unsafe native
calls inside platform modules and explain ownership/lifetimes. Avoid introducing
C++ or OS dependencies into the headless bitmap layer.

Contributions are under the project's MIT license. Be respectful and specific in
reviews; criticize code and reasoning, not people. Security issues should follow
[SECURITY.md](SECURITY.md).
