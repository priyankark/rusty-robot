# Recorded macOS benchmark — 2026-10-27

- `raw.json`: 126 process results, seven timed batches each, 21 operations ×
  three rounds × two implementations. Includes environment and fixture hashes.
- `summary.csv`: medians and round ranges calculated from those samples.
- `optimization-ab.json`: separate 78-process paired Rust before/after run.

These are recorded observations, not measurements of every subsequent commit.
`rust_source_sha256` identifies the source snapshot used by the original runner.
It predates packaging metadata changes. A/B binaries were local artifacts and are
not distributed; their hashes identify them, but the historical before build is
not reconstructible from this repository's initial commit. Current Rust/RobotJS
measurements can be reproduced from source using `scripts/benchmark.py`.

No screenshots, local paths, hostnames, or user text are included. See the
[methodology](../../docs/benchmarks.md) and [interactive report](../../docs/benchmark-report.html).
