# Side-by-side API demonstrations

The Node runner calls the real RobotJS native addon. The Rust runner calls this
repository’s Rust library. Both read the same fixtures, warm up five times,
wait for `go`, then run fixed equal operation counts. Every answer is checked.
They run concurrently; checks and progress IPC are included in elapsed time.
These videos are demonstrations, not replacements for isolated benchmarks.

The recorder screen-grabs an owned Chrome page receiving live progress. It
encodes frames using their wall-clock timestamps, with a short opening and end
hold. No personal desktop contents are captured. Videos are silent; readable
results and descriptions accompany them on the page.

Requirements: Node 24+, Rust, Chrome at the macOS application path, ffmpeg.
Build the pinned RobotJS reference and fixtures with `scripts/benchmark.py`
(see `docs/benchmarks.md`). Then:

```sh
cargo build --release --locked --manifest-path tools/benchmark-demo/Cargo.toml
node tools/benchmark-demo/record.cjs /path/to/robotjs /path/to/fixtures /new/evidence/directory
```

Use a new evidence directory for each recording. Never overwrite published runs.
The gallery uses `docs/demos/2026-10-29`. Regenerate it with
`python3 scripts/demo-report.py`; CI checks the gallery, result agreement, and
video SHA-256 hashes with `--check`.

Cases: 1,000 dense color-result collections; 5,000 absent-color scans;
1,500 late template matches; 5,000 BMP loads. BMP loads check a known pixel as
well as dimensions. Output formats differ (RobotJS BGR24, Rust BGRA32).
