#!/usr/bin/env python3
"""Benchmark results derived from the paired six-round capture experiment."""
import argparse
import json
import statistics
from pathlib import Path
from run import summarize

ROOT=Path(__file__).resolve().parents[2]
DIR=ROOT/'benchmarks/2026-10-28-capture-optimized'
START='<!-- optimized-capture:start -->'
END='<!-- optimized-capture:end -->'

def generate():
    manifest=json.loads((DIR/'manifest.json').read_text())
    assert manifest['completed_utc'] and manifest['rounds']==6
    rows=[json.loads(l) for l in (DIR/'samples.jsonl').read_text().splitlines()]
    summary=summarize(rows)
    assert summary==json.loads((DIR/'summary.json').read_text())
    assert len(summary)==9
    assert all(r['successes']==90 and r['attempts']==90 for r in summary)
    before=json.loads((ROOT/'benchmarks/2026-10-28-rust-ecosystem/manifest.json').read_text())
    assert manifest['baseline_binary_sha256']==before['binary_sha256']
    q=json.loads((DIR/'quality.json').read_text())
    assert q['passed'] and len(q['records'])==14 and q['cursor_test'].startswith('passed:')
    assert q['binary_sha256']==manifest['binary_sha256']
    by={(r['backend'],r['case']):r for r in summary}
    text='''### Repeated captures

**The optimized Rusty Robot beats XCap on repeated full-screen captures on this
Mac. XCap still wins small-region latency.** This is **unreleased source
`1535dc2`**, not the existing crates.io/npm 0.1.2 binaries.

Standalone API comparison on **Apple M6, macOS 27.0, Rust 1.99.1**, against
**XCap 0.9.8**, measured 2026-10-28. No application integrations, network transport,
video encoding, or cached screenshot frames are involved. This is one host,
not a cross-platform ranking. [Enigo + XCap selection and initial baseline](#historical-input-and-capture-baseline).

Each result is an owned CPU RGB buffer at the same requested logical dimensions;
XCap's native 2× image is center-sampled down to match. Rusty Robot also converts
to **sRGB**; XCap's values remain in display color space. Thus dimensions match,
but color processing does not: XCap's numbers exclude sRGB conversion.

| Capture | Previous Rust (ms) | Optimized Rust (ms) | XCap (ms) | Improvement over previous Rust | Current comparison |
| --- | ---: | ---: | ---: | ---: | --- |
'''
    for case,label in [('full','Full display, 1920×1080'),('region100','100×100 region'),('pixel','One pixel')]:
        r=by['rusty-robot',case]['normalized_ms']['median'];b=by['rusty-robot-before',case]['normalized_ms']['median'];x=by['xcap',case]['normalized_ms']['median']
        comparison=f'Rust {x/r:.2f}× faster' if r<x else f'XCap {r/x:.2f}× faster'
        text+=f'| {label} | {b:.2f} | {r:.2f} | {x:.2f} | {b/r:.2f}× | {comparison} |\n'
    text+='''
These are medians of **90 measured calls per case/library**, from six fresh
processes each. Every library passed **270/270 measured freshness checks**.
The previous binary is byte-identical to the original benchmark binary, verified
by SHA-256. It was rerun alongside both contenders; improvements above are not
calculated across different experiments.

| Capture | Rust API median / p95 (ms) | XCap API median / p95 (ms) | Rust same-size RGB p95 (ms) | XCap same-size RGB p95 (ms) |
| --- | ---: | ---: | ---: | ---: |
'''
    for case,label in [('full','Full display'),('region100','100×100'),('pixel','One pixel')]:
        r=by['rusty-robot',case];x=by['xcap',case]
        text+=f"| {label} | {r['api_ms']['median']:.2f} / {r['api_ms']['p95']:.2f} | {x['api_ms']['median']:.2f} / {x['api_ms']['p95']:.2f} | {r['normalized_ms']['p95']:.2f} | {x['normalized_ms']['p95']:.2f} |\n"
    text+='''
| Capture | Rust round-median range, same-size RGB (ms) | XCap round-median range (ms) |
| --- | ---: | ---: |
'''
    for case,label in [('full','Full display'),('region100','100×100'),('pixel','One pixel')]:
        r=by['rusty-robot',case]['normalized_ms'];x=by['xcap',case]['normalized_ms']
        text+=f"| {label} | {r['min_round_median']:.2f}–{r['max_round_median']:.2f} | {x['min_round_median']:.2f}–{x['max_round_median']:.2f} |\n"
    text+='''
**Cold calls matter too.** The first full-display API call in each fresh process
includes first-use capture/render initialization, but excludes process startup
and XCap monitor discovery. These six samples are separate from the warm table:

| First API call | Median (ms) | Range (ms) |
| --- | ---: | ---: |
'''
    for backend,label in [('rusty-robot-before','Previous Rust'),('rusty-robot','Optimized Rust'),('xcap','XCap')]:
        cold=[r['api_ms'] for r in rows if r['backend']==backend and r['case']=='full' and r['trial']==-1]
        assert len(cold)==6
        text+=f'| {label} | {statistics.median(cold):.2f} | {min(cold):.2f}–{max(cold):.2f} |\n'
    for backend,expected in [('rusty-robot',270),('rusty-robot-before',270),('xcap',0)]:
        measured=[r for r in rows if r['backend']==backend and not r['warmup']]
        assert sum(r['exact_srgb'] for r in measured)==expected
    text+='''
The cold Rust/XCap ranges overlap: there is no demonstrated cold-call win.

**What changed:** on macOS 15.2+, screenshots use ScreenCaptureKit's direct region
API. This avoids enumerating shareable windows and copying the full display for
tiny requests. One thread-safe Core Image context reuses compiled rendering state
for sRGB conversion and nearest-neighbor reduction; each call captures fresh pixels
and waits for rendering to finish. Render errors propagate. The macOS 14 path,
permission checks, logical-size API contract, cursor exclusion, and coordinate
metadata remain in place. No deprecated capture API was introduced.

**Quality verification:** the measured red/green fixture matched expected sRGB
within ±3/channel for 270/270 optimized captures and 270/270 previous captures;
XCap returned display-space colors outside that tolerance (0/270 matching the sRGB check). All 270 were valid, fresh captures; this is a color-space difference, not a capture failure. A separate 115-capture
check covers 12 palette colors, a one-logical-pixel checkerboard, opaque tiny
captures, display corners, and 900 pixels under the pointer. Full-display and
tiny-region captures agree exactly at the checked fixture points. The broader
palette differs from the old implementation by at most 2/channel; some colors in
both implementations differ from the fixture's nominal RGB by more than 3 because
of display color conversion. This does **not** establish perfect color accuracy,
HDR quality, or universal bit-for-bit equivalence with old captures.

Five rendering regressions additionally check arbitrary sRGB pixels, orientation,
exact center sampling, alpha, native-density rendering internally, invalid sizes,
and concurrent calls. The public capture API still returns logical-resolution
images on macOS; this update does not add a native-resolution public API.

**Protocol:** two warmups, then 15 measured calls per case/process; six rounds
rotate the three libraries through every execution position twice. Each capture
follows a red/green fixture change and a 40 ms presentation wait outside timing.
The timer ends at an owned CPU buffer. Cases are full display, 100×100, then one
pixel; p95 is nearest rank. Round ranges describe variation, not confidence
intervals. Existing desktop processes remain running. No energy-use, Linux,
Windows, Intel Mac, secondary-display, or persistent-video-stream speed claim is
made. Full input timing remains pending an idle desktop; the initial pilot below
is not upgraded to a larger result by these capture tests.

Reproduce from an unlocked macOS desktop with Screen Recording permission:

```sh
cargo build --release --locked --manifest-path tools/rust-ecosystem-benchmark/Cargo.toml
python3 tools/rust-ecosystem-benchmark/run.py /tmp/capture-new --capture-only --rounds 6
# Add --baseline /path/to/old/driver for the balanced three-library experiment.
python3 tools/rust-ecosystem-benchmark/verify_capture.py /tmp/capture-quality.json
# Optional --cursor also needs Accessibility; it moves/restores only the pointer.
python3 tools/rust-ecosystem-benchmark/optimized_report.py --check
```

For the historical driver, build `tools/rust-ecosystem-benchmark` at `caff367`
in a separate checkout; its core is the original `457a095` source.
The exact binaries measured here are identified in the manifest.

[Raw samples](benchmarks/2026-10-28-capture-optimized/samples.jsonl) ·
[All summaries](benchmarks/2026-10-28-capture-optimized/summary.json) ·
[Environment and hashes](benchmarks/2026-10-28-capture-optimized/manifest.json) ·
[Live quality evidence](benchmarks/2026-10-28-capture-optimized/quality.json) ·
[Scope and omissions](benchmarks/2026-10-28-capture-optimized/notes.json)
'''
    return START+'\n'+text+'\n'+END

def readme_summary():
    rows=json.loads((DIR/'summary.json').read_text())
    by={(r['backend'],r['case']):r for r in rows}
    rust=by['rusty-robot','full']['normalized_ms']['median']
    xcap=by['xcap','full']['normalized_ms']['median']
    return f"""<!-- benchmark-summary:start -->
On one Apple M6 Mac, the unreleased capture implementation returns a full-screen
RGB image in **{rust:.2f} ms versus XCap's {xcap:.2f} ms ({xcap/rust:.2f}× faster)**. XCap remains faster
for small regions. Output dimensions match; color-space processing differs.
These results do not describe the published 0.1.2 binaries or other platforms.

See [benchmark.md](benchmark.md) for metric explanations, results, quality checks,
RobotJS comparisons, raw data, and reproduction steps.
<!-- benchmark-summary:end -->"""


def replace_block(source, start, end, block):
    if source.count(start)!=1 or source.count(end)!=1:
        raise SystemExit(f'Expected exactly one report section: {start}')
    a=source.index(start);b=source.index(end,a)+len(end)
    return source[:a]+block+source[b:]


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args()
    # Validate the raw evidence before updating either document.
    block=generate()
    documents=[(ROOT/'benchmark.md',START,END,block),
               (ROOT/'README.md','<!-- benchmark-summary:start -->',
                '<!-- benchmark-summary:end -->',readme_summary())]
    for path,start,end,block in documents:
        source=path.read_text()
        updated=replace_block(source,start,end,block)
        if args.check:
            if updated!=source:
                raise SystemExit(f'{path.name} drift; run optimized_report.py')
        else:
            path.write_text(updated)
    print('Benchmark report and README summary match paired capture and quality evidence')

if __name__=='__main__':main()
