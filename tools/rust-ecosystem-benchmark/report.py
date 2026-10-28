#!/usr/bin/env python3
"""Regenerate the dated standalone Rust comparison block in the root README."""
import argparse
import json
from pathlib import Path
from run import summarize

ROOT=Path(__file__).resolve().parents[2]
EVIDENCE=ROOT/'benchmarks/2026-10-28-rust-ecosystem'
START='<!-- rust-ecosystem-benchmark:start -->'
END='<!-- rust-ecosystem-benchmark:end -->'

def generate():
    manifest=json.loads((EVIDENCE/'manifest.json').read_text())
    assert 'completed_utc' in manifest, 'Do not publish an interrupted run'
    rows=[json.loads(line) for line in (EVIDENCE/'samples.jsonl').read_text().splitlines()]
    summary=summarize(rows)
    assert summary==json.loads((EVIDENCE/'summary.json').read_text()), 'Summary drift'
    pilot_rows=[json.loads(line) for line in (EVIDENCE/'input-pilot/samples.jsonl').read_text().splitlines()]
    pilot=[r for r in summarize(pilot_rows) if r['category']=='input']
    for backend,profile,expected in [('rusty-robot','default',20),('rusty-robot','zero_configured_delay',20),('enigo','default',18)]:
        group=[r for r in pilot if r['backend']==backend and r['profile']==profile]
        assert len(group)==10 and sum(r['attempts'] for r in group)==20
        assert sum(r['successes'] for r in group)==expected
    assert len(summary)==6
    assert all(r['attempts']==45 and r['successes']==45 for r in summary)
    by={(r['backend'],r['case']):r for r in summary}
    text='''### Rust ecosystem comparison — 2026-10-28

**Enigo + XCap is our reference Rust stack, not a universal “best” claim.**
[Enigo 0.6.1](https://github.com/enigo-rs/enigo) supplies keyboard/mouse input;
[XCap 0.9.8](https://github.com/nashaofu/xcap) supplies screenshots. Their
crates.io recent-download counts at selection were approximately 1.69M and 1.32M,
versus 212K for rdev and 5K for AutoPilot. Adoption motivated the selection; it
is not evidence of speed or reliability. [Selection snapshot](benchmarks/2026-10-28-rust-ecosystem/selection.json).

This compares **standalone desktop automation APIs**, without application
integrations, JavaScript bridges, network transport, or video encoding.
Rusty Robot is source revision `457a095` (package version 0.1.2); competitor
versions and transitive dependencies are pinned in the benchmark Cargo.lock.

#### Screenshots: XCap is faster on this Mac

All times below are milliseconds; **lower is better**. Each cell is **median / p95**.
“Same-size RGB” includes converting each result to packed RGB at the requested
logical dimensions, using center-sampled nearest-neighbor reduction for XCap's
2× Retina image. This equalizes dimensions, **not color space**.

| Capture | Rusty Robot API | XCap API | Rusty Robot same-size RGB | XCap same-size RGB | XCap median advantage |
| --- | ---: | ---: | ---: | ---: | ---: |
'''
    for case,label in [('full','Full display, 1920×1080 logical'),('region100','100×100 logical region'),('pixel','One logical pixel')]:
        r=by['rusty-robot',case]; x=by['xcap',case]
        def cell(row,key):
            v=row[key];return f"{v['median']:.2f} / {v['p95']:.2f}"
        text+=f"| {label} | {cell(r,'api_ms')} | {cell(x,'api_ms')} | {cell(r,'normalized_ms')} | {cell(x,'normalized_ms')} | {r['normalized_ms']['median']/x['normalized_ms']['median']:.2f}× |\n"
    text+='''
Independent-round median ranges for the same-size RGB workload:

| Capture | Rusty Robot range (ms) | XCap range (ms) |
| --- | ---: | ---: |
'''
    for case,label in [('full','Full display'),('region100','100×100 region'),('pixel','One pixel')]:
        r=by['rusty-robot',case]['normalized_ms'];x=by['xcap',case]['normalized_ms']
        text+=f"| {label} | {r['min_round_median']:.2f}–{r['max_round_median']:.2f} | {x['min_round_median']:.2f}–{x['max_round_median']:.2f} |\n"
    for backend,expected in [('rusty-robot',135),('xcap',0)]:
        measured=[r for r in rows if r['backend']==backend and not r['warmup']]
        assert sum(r['exact_srgb'] for r in measured)==expected
    text+='''
Both libraries returned correctly sized, fresh red/green fixture content in
**135/135 measured captures**. Color behavior differs: Rusty Robot matched the
fixture's sRGB primaries within ±3/channel in 135/135; XCap did so in 0/135.
For example, the red `[255, 0, 0]` patch was returned by XCap as `[235, 47, 25]`,
and green `[0, 255, 0]` as `[93, 250, 25]` on this display. Those are display-space
values, not missing screenshots. XCap's macOS path returns raw CGImage RGB bytes
without an attached color profile; Rusty Robot explicitly requests sRGB. No ICC
conversion cost is included for XCap, so this is **not an equal-color-fidelity
speed comparison**. That matters for automation using exact RGB matches.

Rusty Robot's small-region cost is a known limitation: its macOS backend captures
the full display before cropping to avoid partially transparent tiny captures.
XCap uses a direct region capture. This benchmark does not change either algorithm.

#### Keyboard and mouse: pilot only; full timing run pending

An owned AppKit editor/button pilot exercised ASCII, mixed Unicode, 32 emoji,
multiline text, a leading newline, select-all/replace, Return, select-all/delete,
move-and-click, and pointer movement. Results are exact observed text, click
counts, or cursor coordinates—not successful API returns.

| Input configuration | Exact successful deliveries | Measured trials per case |
| --- | ---: | ---: |
| Rusty Robot default 10 ms configured delays | 20/20 | 2 |
| Rusty Robot configured delays set to zero | 20/20 | 2 |
| Enigo defaults | 18/20 | 2 |

Enigo inserted `U+200B` before the leading newline in both measured trials
(`"\\nsecond line"` became `"\\u200b\\nsecond line"`). All its other pilot cases
passed. Rusty Robot retains a minimum 10 ms interval per text batch even when
configured delays are zero; Enigo's macOS text API does not sleep between batches.
The pilot is too small to rank input latency or general reliability. Two larger
input attempts stopped when the owned window lost focus; **neither contributes
to published results**. A full run needs an idle desktop. No failed delivery is
assigned a speedup. [Unchanged pilot evidence](benchmarks/2026-10-28-rust-ecosystem/input-pilot/).

#### Methodology and reproduction

- Apple M6, 32 GiB RAM, macOS 27.0 build 26A425, Rust 1.99.1, release build;
  one main display at 1920×1080 logical / 3840×2160 captured pixels for XCap.
- Capture: three fresh processes per library, alternating backend order;
  two warmups then 15 measured calls per case/process (45 per table row/backend).
  Full display, 100×100, and one-pixel cases run in that order. Capture selection
  is warmed: XCap's monitor is cached as its API intends; Rusty Robot uses its
  normal public capture call. Startup and monitor enumeration are not included.
- The owned window toggles a red/green patch before each capture, waits 40 ms
  outside the timer, and checks the returned patch, dimensions, and scale.
  The capture timer ends when an owned CPU buffer is available; no image files
  are encoded or saved. No private screenshots are retained.
- Input pilot: two warmups and two measured trials per case/profile. Fixture
  reset/focus occurs outside timing; measured input goes through native OS APIs.
  State is polled every 2 ms with a 750 ms timeout and checked again 20 ms after
  success for stability. Raw observed latency includes local control IPC and
  polling, not just OS dispatch. The run stops if its owned window loses focus.
- Medians/p95 use measured successful samples only; round ranges are descriptive,
  not confidence intervals. Background desktop processes were left running.
  The visible Rusty Robot variation is retained rather than selecting its fastest
  round. No cross-platform, Chromium, persistent-video-stream, image-search,
  power-use, or universal performance claim follows from this run. Enigo/XCap
  do not provide a matching bitmap-search API for that part of Rusty Robot.

```sh
cargo build --release --locked --manifest-path tools/rust-ecosystem-benchmark/Cargo.toml
# Requires Screen Recording and a visible desktop. No injected input:
python3 tools/rust-ecosystem-benchmark/run.py /tmp/rust-capture --capture-only
# Requires Accessibility too; leave keyboard/mouse idle with the fixture focused:
python3 tools/rust-ecosystem-benchmark/run.py /tmp/rust-full
python3 -m unittest discover -s tools/rust-ecosystem-benchmark -p 'test_*.py'
python3 tools/rust-ecosystem-benchmark/report.py --check
```

[Raw samples](benchmarks/2026-10-28-rust-ecosystem/samples.jsonl) ·
[Summary with all round medians](benchmarks/2026-10-28-rust-ecosystem/summary.json) ·
[Environment and binary hashes](benchmarks/2026-10-28-rust-ecosystem/manifest.json) ·
[Benchmark source and lockfile](tools/rust-ecosystem-benchmark/)
'''
    return START+'\n'+text+'\n'+END

def main():
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');args=p.parse_args()
    readme=ROOT/'README.md';s=readme.read_text();block=generate()
    if START in s:
        a=s.index(START);b=s.index(END,a)+len(END)
        updated=s[:a]+block+s[b:]
    else: updated=s.replace('## For agents',block+'\n\n## For agents')
    if args.check:
        if updated!=s: raise SystemExit('README benchmark drift; run report.py')
        print('README Rust benchmark matches raw evidence')
    else: readme.write_text(updated)

if __name__=='__main__': main()
