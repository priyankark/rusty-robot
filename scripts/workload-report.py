#!/usr/bin/env python3
"""Validate screenshot workload evidence and embed all comparisons in the article."""
import argparse
import hashlib
import json
import statistics
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()


def write(path, content):
    if args.check:
        assert path.read_text() == content, f'Regenerate with scripts/workload-report.py: {path}'
    else:
        path.write_text(content)


CASES = {
    'dashboard': {
        'title': 'Find every Ready badge',
        'task': 'An automation needs all the clickable Ready targets on an order dashboard. Search 60 screenshots containing 2,400 cards and return all 1,527 badge rectangles.',
        'method': 'Both public find-all-color APIs collect green pixel positions. The same row-major grouping steps turn those positions into rectangles. Search, result allocation, and grouping are timed.',
        'check': 'Every rectangle matches the expected geometry recorded independently from the fixture DOM: all 1,527, in the same order, in every run.',
        'reason': 'This task returns many pixel positions. RobotJS creates JavaScript objects; Rust returns a vector. Grouping then runs in JavaScript versus compiled Rust. Those allocation and loop costs are part of this API workflow.',
        'video': 'dashboard',
    },
    'alerts': {
        'title': 'Find failed deployments',
        'task': 'A monitoring automation checks 60 screenshots, each with 40 services. Twelve have a red failure badge; 48 have none. Find the first failure, or confirm that no failure color is present.',
        'method': 'Both public first-color-match APIs scan for the same exact red. A found position becomes a known-size 100 × 28 badge rectangle. Search and this small result construction are timed.',
        'check': 'All 60 answers match independently recorded fixture geometry, including every no-match answer. This checks both detection and absence; it does not infer badge dimensions from pixels.',
        'reason': 'Unlike the Ready-badge task, this returns at most one rectangle per image. The advantage is in this exact-color scan workload, rather than building a large list of JavaScript objects. Forty-eight no-match images require full scans.',
        'video': 'workload',
    },
    'heatmap': {
        'title': 'Count passing tests in a heatmap',
        'task': 'Read 60 test heatmaps containing 648,000 tiles in total. Each solid green tile represents a passing check. Recover the passing count for every image: 547,560 across the batch.',
        'method': 'Both public color-count APIs count the same exact green pixels. Each generated tile is exactly 10 × 7 pixels. Only counting and retaining the raw counts are timed; division by 70 happens afterward.',
        'check': 'Every raw pixel count must equal the independently generated passing-test count × 70. Checking raw counts prevents integer division from hiding a small error. All 60 counts agree in every run.',
        'reason': 'Each API returns one count, so large result arrays cannot explain this result. It measures repeated full-image exact-color counting. The tile-to-test conversion assumes this known layout; it is not a general heatmap parser.',
        'video': 'workload',
    },
}

data = {}
for case, info in CASES.items():
    folder = ROOT / f'docs/workflows/2026-10-30-{case}'
    manifest = json.loads((folder / 'manifest.json').read_text())
    fixtures = json.loads((folder / 'fixtures.json').read_text())
    expected = [f['expected'] for f in fixtures]
    assert len(fixtures) == 60
    assert all(f['width'] == 1920 and f['height'] == 1080 for f in fixtures)
    assert len({f['sha256'] for f in fixtures}) == 60
    targets = sum(expected) if case == 'heatmap' else sum(map(len, expected))
    assert targets == {'dashboard': 1527, 'alerts': 12, 'heatmap': 547560}[case]
    if case == 'alerts':
        assert sum(not e for e in expected) == 48 and all(len(e) <= 1 for e in expected)
    for filename, key in [('screenshots.zip', 'corpus_sha256'), (info['video'] + '.mp4', 'video_sha256')]:
        assert hashlib.sha256((folder / filename).read_bytes()).hexdigest() == manifest[key]
    with zipfile.ZipFile(folder / 'screenshots.zip') as archive:
        assert json.loads(archive.read('fixtures.json')) == fixtures
        assert {n for n in archive.namelist() if n.endswith('.png')} == {f['name'] + '.png' for f in fixtures}

    def verify(run):
        assert run['verified'] and run['results'] == expected
        assert run['screenshots'] == 60 and run['targets'] == targets
        assert run['seconds'] > 0
        if case == 'heatmap':
            assert run['raw_pixel_counts'] == [e * 70 for e in expected]

    assert len(manifest['runs']) == 12
    for number in range(1, 7):
        assert [r['language'] for r in manifest['runs'] if r['round'] == number] == (
            ['robotjs', 'rust'] if number % 2 else ['rust', 'robotjs'])
    for row in manifest['runs']:
        run = json.loads((folder / row['file']).read_text())
        verify(run)
        assert run['seconds'] == row['seconds']
    for language in ['robotjs', 'rust']:
        video = json.loads((folder / f'{language}-video.json').read_text())
        verify(video['complete'])
        progress = [e for e in video['events'] if e['type'] == 'progress']
        assert len(progress) == 60
        assert [e['boxes' if case == 'dashboard' else 'result'] for e in progress] == expected
    stats = {}
    for language in ['robotjs', 'rust']:
        samples = [r['seconds'] * 1000 for r in manifest['runs'] if r['language'] == language]
        stats[language] = dict(median_ms=statistics.median(samples), min_ms=min(samples), max_ms=max(samples))
    ratio = stats['robotjs']['median_ms'] / stats['rust']['median_ms']
    summary = dict(results=stats, ratio=ratio, screenshots=60, targets=targets, verified_runs=12)
    write(folder / 'summary.json', json.dumps(summary, indent=2) + '\n')
    data[case] = (manifest, summary)


def embed(name, parts):
    article = ROOT / 'docs/building-rusty-robot.html'
    source = article.read_text()
    begin, end = f'<!-- BEGIN {name} -->', f'<!-- END {name} -->'
    assert source.count(begin) == source.count(end) == 1, f'Missing or duplicate {name} markers'
    start, finish = source.index(begin), source.index(end) + len(end)
    write(article, source[:start] + begin + '\n' + '\n'.join(parts) + '\n' + end + source[finish:])


parts = ['''<section id="workloads" aria-labelledby="workload-heading">
<p class="chapter">03 · Put the gains to work</p><h2 id="workload-heading">What faster image analysis buys an automation</h2>
<p>We tested three tasks on generated screenshots with known colors and layouts: collect Ready badges, find a failed deployment, and count passing test tiles. Both libraries received identical pixels already loaded into memory; <strong>capture and image loading are excluded.</strong></p>
<p>Each time is the median of six 60-image batches, with every answer checked. A 2× speedup means half the analysis time. These are comparisons of the public APIs on one Mac.</p>
<details class="evidence"><summary>Shared method, machine, and limits</summary><p>Both receive identical preloaded 1920 × 1080 BGRA32 buffers. Each fresh process completes and verifies a full warmup. Six rounds alternate library order. No recording runs during measurement; outputs are checked after the timer. There are no artificial sleeps in the timed analysis.</p><p>Apple M6, macOS 27.0, Node 24.22.0, Rust 1.99.1 release builds; RobotJS commit <code>0af6073</code>. Allocation and public API wrapper costs are included. Capture, PNG decoding, disk reads, and warmup are excluded.</p><p>Each corpus contains 60 distinct images of one generated design. We selected these cases to illustrate image-analysis wins; they are not a representative sample of all automation. We did not compare custom JavaScript buffer scans. Results are specific to this machine and these public APIs, and their ratios should not be averaged.</p></details>
<p class="video-guide"><strong>Watching the runs:</strong> RobotJS is left, Rust is right. Separate takes are aligned at task start; the finished side holds. Video clocks include recording overhead and differ from the medians. Playback starts at 1×; ¼× slows it for inspection.</p>''']
for case, info in CASES.items():
    manifest, result = data[case]
    stats = result['results']
    base = f'workflows/2026-10-30-{case}/'
    saved = stats['robotjs']['median_ms'] - stats['rust']['median_ms']
    impact = {
        'dashboard': f'Rust saved {saved:.0f} ms across the batch, returning all 1,527 target rectangles in about 49 ms.',
        'alerts': f'Rust saved {saved:.2f} ms across 60 images, returning the same 12 failures and 48 no-match answers.',
        'heatmap': f'Rust saved {saved:.2f} ms across the batch, counting the same 547,560 passing tiles.',
    }[case]
    parts.append(f'<section id="workload-{case}"><h3>{info["title"]}</h3><p>{info["task"]}</p><div class="explorer" role="figure" aria-label="Median batch time for {info["title"].lower()}"><p class="workload-result"><strong>{result["ratio"]:.2f}× faster</strong> in this analysis task</p><p class="chart-note">60-image batch · milliseconds · shorter is faster</p>')
    for language, label, cls in [('robotjs', 'RobotJS', 'base'), ('rust', 'Rust', 'rust')]:
        value = stats[language]['median_ms']
        parts.append(f'<div class="bar-line"><span>{label}</span><div class="bar-track" aria-hidden="true"><div class="bar-fill {cls}" style="width:{value / stats["robotjs"]["median_ms"] * 100:.4f}%"></div></div><output>{value:.2f} ms</output></div>')
    parts.append(f'''</div><p>{impact}</p><figure class="demo-card"><video id="video-{case}" aria-label="{info['title']}, RobotJS and Rust side by side" controls playsinline preload="none" poster="{base}{info['video']}.png"><source src="{base}{info['video']}.mp4" type="video/mp4"><a href="{base}{info['video']}.mp4">Download video</a></video><div class="links"><button type="button" data-video="video-{case}" data-rate="1">Replay at 1×</button><button type="button" data-video="video-{case}" data-rate="0.25">Inspect at ¼×</button></div><figcaption id="video-{case}-status" aria-live="polite">Playback: original speed (1×). Separate recorded takes; not the measured medians.</figcaption></figure><p>{info['reason']}</p><details class="evidence"><summary>What this test does, how we checked it, and all six runs</summary><p><strong>Timed work:</strong> {info['method']}</p><p><strong>Correctness:</strong> {info['check']}</p><div class="table-wrap" tabindex="0"><table><caption>Batch time in milliseconds. Every output verified.</caption><thead><tr><th scope="col">Round</th><th scope="col">RobotJS</th><th scope="col">Rust</th></tr></thead><tbody>''')
    for number in range(1, 7):
        rows = {r['language']: r for r in manifest['runs'] if r['round'] == number}
        parts.append(f'<tr><th scope="row">{number}</th>' + ''.join(f'<td><a href="{base}{rows[lang]["file"]}">{rows[lang]["seconds"] * 1000:.2f}</a></td>' for lang in ['robotjs', 'rust']) + '</tr>')
    parts.append('</tbody></table></div><p>' + '; '.join(f'{label} range: {stats[lang]["min_ms"]:.2f}–{stats[lang]["max_ms"]:.2f} ms' for lang, label in [('robotjs', 'RobotJS'), ('rust', 'Rust')]) + '.</p>')
    parts.append(f'<div class="links"><a href="{base}summary.json">Computed summary</a><a href="{base}manifest.json">Environment, runs, and hashes</a><a href="{base}screenshots.zip">All 60 input screenshots</a><a href="{base}fixtures.json">Expected outputs and input hashes</a><a href="{base}{info["video"]}.mp4" download>Video</a><a href="https://github.com/priyankark/rusty-robot/tree/main/tools/benchmark-demo">Source and reproduction steps</a></div></details></section>')
parts.append('</section>')
embed('WORKLOAD COMPARISONS', parts)

typing = json.loads((ROOT / 'docs/workflows/2026-10-30-controlled/summary.json').read_text())
parts = ['''<div class="table-wrap" tabindex="0"><table><caption>Same form, 8 records, 24 fields, 769 ASCII characters · median seconds</caption><thead><tr><th scope="col">Typing profile</th><th scope="col">RobotJS</th><th scope="col">Rust</th><th scope="col">Result</th></tr></thead><tbody>''']
for profile, label, verdict in [('zero', 'Requested keyboard delay: 0 ms', 'RobotJS faster'), ('paced', 'Matched 6,000 characters/minute', 'Effectively tied')]:
    stats = typing['profiles'][profile]
    assert all(s['passed'] == s['attempts'] == 6 for s in stats.values())
    parts.append(f'<tr><th scope="row">{label}</th><td>{stats["robotjs"]["median"]:.3f} s</td><td>{stats["rust"]["median"]:.3f} s</td><td>{verdict}</td></tr>')
parts.append('''</tbody></table></div><details class="evidence"><summary>Typing videos, method, and the earlier pacing correction</summary><p>Both libraries entered the same records through native input into an owned Chrome form. Mouse delay was 10 ms and key-tap delay was zero for both. One warmup per backend and profile preceded six alternating measured trials, without recording. Every saved field matched in every trial.</p>''')
for profile, label in [('zero', 'Zero requested keyboard delay'), ('paced', 'Matched typing rate')]:
    base = f'workflows/2026-10-30-controlled/{profile}'
    parts.append(f'<figure class="demo-card"><h3>{label}</h3><video controls playsinline preload="none" aria-label="{label}, RobotJS and Rust side by side" poster="{base}.png"><source src="{base}.mp4" type="video/mp4"><a href="{base}.mp4">Download video</a></video><figcaption>Separate illustrative recordings after measurement; video clocks are not the table medians.</figcaption></figure>')
parts.append('''<p>The <a href="default-workflow-demos.html">earlier default-delay demo</a> favored Rust because RobotJS waited per character while Rust batched text. Its typing pace was not matched; the table above uses explicit timing profiles.</p><p><a href="typing-comparison.html">Full protocol and all typing trials</a> · <a href="workflows/2026-10-30-controlled/manifest.json">Raw evidence manifest</a>. RobotJS capture failed on this host during the earlier default workflow run; that failure is excluded from speed ratios.</p></details>''')
embed('TYPING COMPARISON', parts)
print('Verified 36 screenshot batches, 6 recorded takes, exact outputs, corpus/video hashes, and both generated story sections.')
