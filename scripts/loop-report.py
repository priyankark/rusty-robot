#!/usr/bin/env python3
"""Validate fresh-capture/input receipts and generate the complete-loop report."""
import argparse
import hashlib
import json
import math
import statistics as stats
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FOLDER = ROOT / 'docs/workflows/2026-10-31-loop'
BASE = 'workflows/2026-10-31-loop/'
args = argparse.ArgumentParser(description=__doc__)
args.add_argument('--check', action='store_true')
CHECK = args.parse_args().check

def write(path, text):
    if CHECK:
        assert path.read_text() == text, f'Regenerate with scripts/loop-report.py: {path}'
    else:
        path.write_text(text)

def close_color(a, b):
    return a == b

def describe(values):
    assert values and all(math.isfinite(v) and v >= 0 for v in values)
    return dict(median_ms=stats.median(values), p95_ms=sorted(values)[math.ceil(len(values)*.95)-1], mean_ms=stats.mean(values))

m = json.loads((FOLDER / 'manifest.json').read_text())
if (FOLDER/'diagnostics.json').exists():
    diagnostic=json.loads((FOLDER/'diagnostics.json').read_text())
    assert diagnostic['standalone']['permission'] and diagnostic['node_context']['permission']
    assert all(c['returned_image'] for c in diagnostic['standalone']['captures'])
    assert all(not c['returned_image'] for c in diagnostic['node_context']['captures'])
    for d in diagnostic['robotjs'].values():
        assert d['permission'] and not d['returned_image'] and d['source_clean']
        assert d['source_commit']==m['robotjs_commit']
    assert diagnostic['robotjs']['rebuilt']['binary_sha256']==m['robotjs_binary_sha256']
    assert diagnostic['source_sha256']==hashlib.sha256((ROOT/'tools/benchmark-demo/capture-diagnostic.c').read_bytes()).hexdigest()
else:
    raise AssertionError('Run separate capture diagnostics before publishing')
assert not m['pilot'] and len(m['cases']) == 5
assert m['robotjs_commit'] == '0af6073e1f1bcff865ffe52801c71647829cdc1c'
assert hashlib.sha256((FOLDER/'app.png').read_bytes()).hexdigest() == m['app_sha256']
for name, expected in m['sources'].items():
    if not name.startswith('target/'):
        assert hashlib.sha256((ROOT/'tools/benchmark-demo'/name).read_bytes()).hexdigest() == expected, name

codes=[t['request']['code'] for r in m['runs'] for t in r['trials']]
assert len(codes)==len(set(codes)) and all(0<c<65536 for c in codes)
summary = {'date': m['date'], 'source_commit': m['source_commit'], 'cases': {}}
for case in m['cases']:
    assert len(case['code_points']) == (0 if case['id']=='pixel' else 16)
    assert all(0<=p[0]<case['rect'][2] and 0<=p[1]<case['rect'][3] for p in case['code_points'])
    by_language = {}
    for lang in ('robotjs', 'rust'):
        runs = [r for r in m['runs'] if r['case'] == case['id'] and r['language'] == lang]
        assert runs
        unavailable = any('unavailable' in r for r in runs)
        assert [r['round'] for r in runs] == ([1] if unavailable else list(range(1, 7)))
        trials = []
        for run in runs:
            assert run['probe']['accessibility'] is True and run['probe']['capture'] is True
            assert len(run['trials']) == (5 if unavailable else 25)
            assert sum(t['warmup'] for t in run['trials']) == 5
            for t in run['trials']:
                req = t['request']; receipt = t['receipt']
                assert receipt['active'] and not receipt['focusLost']
                for key in ('rect', 'sample', 'op', 'delay', 'code_points'):
                    assert req[key] == case[key]
                assert receipt['code'] == req['code']
                assert receipt['id'] == t['id'] and receipt['before'] == req['before'] and receipt['after'] == req['after']
                assert not close_color(req['before'], req['after'])
                if not t['ok']:
                    assert t['error'] and not t['verified']
                else:
                    result = t['result']
                    assert result['width'] == req['rect'][2] and result['height'] == req['rect'][3]
                    assert result['scale_x'] == result['scale_y'] == 1
                    assert close_color(result['first_color'], req['before'])
                    assert result['first_code'] == (req['code'] if req['code_points'] else None)
                    assert result['capture_ms'] > 0
                    if case['op'] == 'capture':
                        assert t['verified'] and not receipt['clicks'] and not receipt['complete']
                    else:
                        loop = result['loop']
                        assert loop['polls'] >= 1
                        stages = [result['capture_ms']] + [loop[k] for k in ('find_ms', 'input_ms', 'observe_ms')]
                        assert all(math.isfinite(v) and v >= 0 for v in stages)
                        assert loop['total_ms'] >= sum(stages)
                        delivered = receipt['complete'] and len(receipt['clicks']) == 1 and receipt['clicks'][0]['trusted']
                        assert t['verified'] == (loop['complete'] and delivered)
                        if t['verified']:
                            assert close_color(loop['last_color'], req['after'])
                            assert loop['last_code'] == req['code']
                            click = receipt['clicks'][0]; b = receipt['button']
                            assert b['x'] <= click['x'] < b['x']+b['width'] and b['y'] <= click['y'] < b['y']+b['height']
                            assert loop['click'] == [m['geometry']['x']+click['x'], m['geometry']['y']+click['y']]
                        else:
                            assert not loop['complete'] and loop['total_ms'] >= 1000
                if not t['warmup']:
                    trials.append(t)
        valid = [t for t in trials if t['verified']]
        item = dict(attempts=len(trials), successful=len(valid), failed=len(trials)-len(valid),
                    warmup_attempts=sum(t['warmup'] for r in runs for t in r['trials']),
                    warmup_failures=sum(t['warmup'] and not t['verified'] for r in runs for t in r['trials']),
                    unavailable=unavailable)
        if unavailable:
            assert not trials and all(not t['ok'] for r in runs for t in r['trials'])
        else:
            assert len(trials) == 120
        def timing(t):
            return t['result']['capture_ms'] if case['op'] == 'capture' else t['result']['loop']['total_ms']
        if valid:
            item.update(describe([timing(t) for t in valid]))
            item['round_medians_ms'] = [stats.median([timing(t) for t in r['trials'] if not t['warmup'] and t['verified']]) for r in runs]
            if case['op'] == 'loop':
                item['stages'] = {key: describe([t['result']['capture_ms'] if key == 'capture_ms' else t['result']['loop'][key] for t in valid]) for key in ('capture_ms', 'find_ms', 'input_ms', 'observe_ms')}
                item['median_poll_count'] = stats.median(t['result']['loop']['polls'] for t in valid)
        by_language[lang] = item
    summary['cases'][case['id']] = by_language

# The recorded run has no valid RobotJS capture baseline. This is an explicit
# availability result, never a divide-by-zero or timeout-derived speed ratio.
assert all(v['robotjs']['unavailable'] for v in summary['cases'].values())
write(FOLDER/'summary.json', json.dumps(summary, indent=2)+'\n')

labels = {'pixel': 'One pixel', 'region': '100 × 100 region', 'window': '1200 × 800 window', 'loop-zero': 'Complete loop, 0 ms mouse delay', 'loop-default': 'Complete loop, 10 ms mouse delay'}
rows = []
mdrows = []
for key, label in labels.items():
    r = summary['cases'][key]['rust']
    rows.append(f'<tr><th scope="row">{label}</th><td>{r["successful"]}/{r["attempts"]}</td><td>{r["median_ms"]:.2f}</td><td>{r["p95_ms"]:.2f}</td><td>{r["failed"]}</td></tr>')
    mdrows.append(f'| {label} | {r["successful"]}/{r["attempts"]} | {r["median_ms"]:.2f} ms | {r["p95_ms"]:.2f} ms | {r["failed"]} |')
default = summary['cases']['loop-default']['rust']
zero = summary['cases']['loop-zero']['rust']
stage_labels = {'capture_ms': 'First capture', 'find_ms': 'Find target', 'input_ms': 'Move and click', 'observe_ms': 'Capture visible response'}
bars = []
for key, label in stage_labels.items():
    value = default['stages'][key]['mean_ms']
    bars.append(f'<div class="bar-line"><span>{label}</span><div class="bar-track" aria-hidden="true"><div class="bar-fill rust" style="width:{value/default["mean_ms"]*100:.4f}%"></div></div><output>{value:.2f} ms</output></div>')

html = f'''<section aria-labelledby="capture"><p class="chapter">05 · Timing the complete interaction</p><h2 id="capture">From a screenshot to a confirmed action</h2>
<p>This test measures how long a desktop automation library takes to find and click a button, including capturing the screen and confirming the application’s response. The timer starts at the first screenshot request and stops when a later screenshot shows the result of the click.</p>
<p>We built a local test app in Chrome that simulates a review queue. An Approve button moves among twelve cards. Each attempt starts by taking a fresh screenshot, finds the button by its color, moves the mouse and clicks, then takes more screenshots until the app shows approval. The app also records whether the button received exactly one click delivered through the operating system. A successful API return alone does not count as a completed task.</p>
<p>With the library’s default 10 ms pause after each mouse operation, Rust completed <strong>{default['successful']} of {default['attempts']} measured interactions</strong>. The median was <strong>{default['median_ms']:.2f} ms</strong>, and 95% of successful interactions finished within <strong>{default['p95_ms']:.2f} ms</strong>. These are complete local interactions, including capture and the visible response, in a controlled test app.</p>
<p><strong>RobotJS could not complete the screenshot step on this Mac.</strong> We tested the same three capture sizes and two interaction settings listed in the table. For each, RobotJS failed five initial capture attempts despite granted permissions, so we stopped that case. A fresh build of the same RobotJS source also failed. Without a successful RobotJS result, this experiment cannot establish which library completes the task faster.</p>
<figure class="story-figure"><div class="table-wrap" tabindex="0"><table><caption>Fresh captures and complete interactions · October 31 UTC · Rust results · times in ms</caption><thead><tr><th scope="col">Task</th><th scope="col">Completed</th><th scope="col">Median</th><th scope="col">p95</th><th scope="col">Failed</th></tr></thead><tbody>{''.join(rows)}</tbody></table></div><figcaption>Each row has 120 measured attempts across six fresh processes. Timings describe successful attempts only. RobotJS failed all five initial capture attempts for each task; further attempts were skipped. Startup, resets and independent receipt checks are outside timing. No recording ran during measurement.</figcaption></figure>
<p><strong>At zero delay, most target clicks failed.</strong> Setting the delay to 0 ms removes the library’s pause after mouse operations. In {zero['failed']} of {zero['attempts']} measured attempts, the screenshot and target search succeeded and the input APIs returned, but the Approve button recorded no click. The attempt failed when no visible result arrived within the one-second deadline. With the default delay, all {default['attempts']} attempts completed. The precise cause of the zero-delay failures has not been confirmed. The faster median for its successful attempts must be read alongside its failure count.</p>
<div class="explorer" role="figure" aria-label="Where the 10 ms delay complete interaction spends time"><h3>Where the interaction spends its time</h3><p class="chart-note">Mean milliseconds per successful interaction at 10 ms mouse delay. Each bar uses the complete loop’s mean as its scale. Frame validation and bookkeeping make up the small remainder.</p>{''.join(bars)}</div>
<p>The target search is a small part of this loop. Capturing pixels, delivering input and waiting for a visible frame take most of the time. The <a href="#app-github">49× GitHub diff-analysis speedup</a> measures searching and grouping matches in 60 screenshots already in memory. It excludes taking screenshots and interacting with an app, so it cannot predict the speedup for this complete task.</p>
<details class="evidence"><summary>What we checked, and what the result means</summary><p>The screenshot covers a dedicated 1200 × 800 test window on an Apple M6 Mac running macOS {m['os']}, Node {m['node']} and a Rust release build. The app alternates grayscale states and draws a unique 16-bit black-and-white trial ID. Both drivers check the exact state color and ID before acting, then require that same ID with the completion color. The one-pixel capture can check only the alternating state, because its buffer cannot contain the ID. Both use exact magenta search and map bitmap coordinates to desktop coordinates. DOM data verifies delivery after the timer stops; it does not choose the target or end the timer.</p><p>We separately request 1 × 1, 100 × 100 and 1200 × 800 fresh captures. The interaction profiles use the same window and explicitly match each library’s mouse delay at either 0 or 10 ms per call. Completion requires another screenshot showing the changed marker. Failed interactions retain their recorded app events and timeout timings. Five excluded warmups precede each twenty-attempt measured round; library order alternates when both backends are available.</p><p>A standalone C diagnostic returned images through the legacy CoreGraphics entry point used by RobotJS. A minimal Node addon calling that same entry point failed, as did a fresh build of RobotJS. This narrows the observed failure to the Node capture context on this machine; it does not prove that the API is universally broken or that we know the underlying cause. The October 27 successful RobotJS captures remain historical results and are not mixed into this run.</p><p>This establishes Rust’s latency and observed reliability for this local approval task. A matched speed comparison still requires successful captures from both libraries in the same run. Network-backed apps, other OSs, and other displays need their own measurements.</p><div class="links"><a href="{BASE}summary.json">Computed results</a><a href="{BASE}manifest.json">Every attempt and app receipt</a><a href="{BASE}diagnostics.json">Capture diagnostics</a><a href="{BASE}app.png">The test application</a><a href="https://github.com/priyankark/rusty-robot/tree/main/tools/benchmark-demo#fresh-capture-and-the-complete-interaction-loop">Source and reproduction</a></div></details>
<details class="evidence"><summary>How version 0.1.3 reduced screenshot overhead</summary><p>The first macOS backend captured a whole display and cropped afterward. Version 0.1.3 requests the rectangle directly through ScreenCaptureKit on macOS 15.2 and newer, retains the earlier path for macOS 14, and reuses a thread-safe Core Image rendering context. Each call still returns a fresh owned sRGB buffer after rendering completes.</p><p>In the separate October 28 Rust before/after run, full-display capture fell from 50.98 to 22.31 ms and 100 × 100 capture from 44.20 to 9.57 ms. Those measurements use a different protocol and are not combined with old RobotJS times. A separate 115-capture check covered freshness, palette colors, checkerboards, tiny regions, corners and cursor exclusion. <a href="https://github.com/priyankark/rusty-robot/blob/v0.1.3/benchmarks/2026-10-28-capture-optimized/quality.json">Quality evidence</a>.</p></details>
</section>'''
article = ROOT/'docs/building-rusty-robot.html'
text = article.read_text()
begin, end = '<!-- BEGIN FULL LOOP -->', '<!-- END FULL LOOP -->'
assert text.count(begin) == text.count(end) == 1
text = text[:text.index(begin)]+begin+'\n'+html+'\n'+end+text[text.index(end)+len(end):]
write(article, text)

md = f'''## Fresh capture and complete interactions (2026-10-31 UTC)

This benchmark measures the time from requesting a screenshot to seeing an
application respond to a click. A local test app in Chrome simulates a review
queue, with an Approve button that moves among twelve cards. Rust captures the
screen, finds the button by color, sends a native mouse click, and captures again
until approval appears. The app independently checks that the button received
exactly one trusted click. Returning successfully from an input API is not enough.

**At the normal 10 ms mouse delay, Rust completed {default['successful']}/{default['attempts']} measured
interactions: median {default['median_ms']:.2f} ms, p95 {default['p95_ms']:.2f} ms.** At zero requested delay,
{zero['failed']}/{zero['attempts']} attempts failed: screenshots and target searches succeeded, but
the button recorded no click and the one-second response deadline expired.
The precise cause of the zero-delay failures has not been confirmed.

| Task | Completed | Median | p95 | Failed |
| --- | --- | --- | --- | --- |
{chr(10).join(mdrows)}

Capture rows time the capture call. Loop rows include initial capture, freshness
verification, exact target search, coordinate conversion, mouse movement/click,
and fresh screenshot polling until the changed marker appears. Timings describe
successful attempts only; failures remain in the raw data. p95 is the nearest-rank
95th percentile. Six fresh processes each run five excluded warmups and twenty
measured attempts. Resets, two animation frames plus 50 ms settling, ownership
checks, host IPC and independent DOM receipts are outside timing. Both drivers
check exact grayscale states, a unique 16-bit black-and-white trial ID
(except the one-pixel case), and the exact magenta target.

**RobotJS failed five initial capture attempts for each of the three capture
sizes and two interaction settings.** Further attempts were skipped. Without a
successful RobotJS result, this test cannot establish which library is faster.
Permissions passed. A clean rebuild of the pinned addon also failed. A standalone
C capture succeeded; a minimal Node addon calling the same legacy API failed.
That narrows the context without establishing the underlying cause. The older
successful RobotJS measurements are not substituted for missing current samples.

The [49× GitHub diff-analysis speedup](#real-app-image-analysis-versus-robotjs-2026-10-30)
measures searching and grouping matches in 60 screenshots already in memory. This complete interaction spends most of its time capturing,
delivering
input and waiting for the app to render. It does not establish a general Rust
automation speedup. Hardware: {m['cpu']}, macOS {m['os']}, Node {m['node']}; Rust release.

[Full report and stage chart](https://priyankark.github.io/rusty-robot/building-rusty-robot.html#capture)
· [Every attempt](docs/workflows/2026-10-31-loop/manifest.json)
· [Summary](docs/workflows/2026-10-31-loop/summary.json)
· [Diagnostics](docs/workflows/2026-10-31-loop/diagnostics.json)
· [Reproduction](tools/benchmark-demo/README.md#fresh-capture-and-the-complete-interaction-loop)
'''
report=ROOT/'benchmark.md'; text=report.read_text()
begin,end='<!-- full-loop:start -->','<!-- full-loop:end -->'
assert text.count(begin)==text.count(end)==1
write(report,text[:text.index(begin)]+begin+'\n'+md+end+text[text.index(end)+len(end):])
print('Validated capture freshness, all input receipts, failures, timings and generated full-loop reports.')
