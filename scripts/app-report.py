#!/usr/bin/env python3
"""Verify real-app screenshot evidence and render the article's two app demos."""
import argparse
import hashlib
import json
import statistics
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check',action='store_true')
args=parser.parse_args()

def sha(data):return hashlib.sha256(data).hexdigest()
def write(path,text):
    if args.check:assert path.read_text()==text,f'Regenerate with scripts/app-report.py: {path}'
    else:path.write_text(text)

CASES={
    'github':('Locate added code in a GitHub diff','Find the green added-code regions in 60 captured views of this project’s merged pull request. The output is 64 visible regions, including correct empty answers when no added code is in the scan area.','The diff contains large colored blocks. RobotJS creates a JavaScript object for each matching pixel; Rust processes a vector of points. The gain includes scanning, allocation, and grouping those pixels into regions.'),
    'vscode':('Locate search highlights in VS Code','Find highlighted code across 60 captured editor views, using six search queries against Rust source. Both implementations return the same 106 regions. Adjacent highlighted rows share a rectangle.','The highlights occupy much less of the screen, so there are fewer pixel objects to create. Rust still finishes sooner, but the absolute saving is about 31 ms for the whole batch.'),
}
parts=['''<section id="workloads" aria-labelledby="app-heading"><p class="chapter">03 · Real application screens</p><h2 id="app-heading">Where the speedups become visible</h2>
<p>We captured VS Code and a GitHub pull-request diff through Rusty Robot, then ran the same color-search and region-grouping steps through both public APIs. These demos analyze actual app screens; <strong>navigation, capture, and image loading are outside the timer.</strong></p>
<p>Each result is the median of six 60-image batches. Every returned rectangle was checked. The videos are separate recorded runs, with RobotJS on the left and Rust on the right; recording overhead makes their clocks differ from the medians.</p>''']
for case,(title,task,reason) in CASES.items():
    folder=ROOT/f'docs/workflows/2026-10-30-{case}'
    manifest=json.loads((folder/'manifest.json').read_text())
    fixtures=json.loads((folder/'fixtures.json').read_text())
    capture=json.loads((folder/'capture.json').read_text())
    assert sha((folder/'capture.json').read_bytes())==manifest['sha256']['capture_provenance']
    assert len(fixtures)==60 and len({f['sha256'] for f in fixtures})==60
    assert all(f['width']==1200 and f['height']==800 and f['oracle_verified'] for f in fixtures)
    expected=[f['expected'] for f in fixtures]
    targets=sum(map(len,expected));assert targets=={'vscode':106,'github':64}[case]
    assert capture['pixel_oracle']['verified_frames']==60
    assert sha((folder/'workload.mp4').read_bytes())==manifest['video_sha256']
    archive=folder/'screenshots.zip'
    assert sha(archive.read_bytes())==manifest['corpus_sha256']
    names={f['name']+ext for f in fixtures for ext in ['.png','.bgra']}|{'fixtures.json','capture.json'}
    with zipfile.ZipFile(archive) as z,tempfile.TemporaryDirectory() as temp:
        assert set(z.namelist())==names
        assert json.loads(z.read('fixtures.json'))==fixtures
        assert json.loads(z.read('capture.json'))==capture
        for name in names:(Path(temp)/name).write_bytes(z.read(name))
        subprocess.run([sys.executable,str(ROOT/'tools/benchmark-demo/app-oracle.py'),temp,'--check'],check=True,stdout=subprocess.DEVNULL)
    def verify(run):
        assert run['verified'] and run['screenshots']==60 and run['targets']==targets
        assert run['seconds']>0 and run['results']==expected
    assert len(manifest['runs'])==12
    for round_no in range(1,7):
        assert [r['language'] for r in manifest['runs'] if r['round']==round_no]==(['robotjs','rust'] if round_no%2 else ['rust','robotjs'])
    for row in manifest['runs']:
        run=json.loads((folder/row['file']).read_text());verify(run);assert row['seconds']==run['seconds']
    for language in ['robotjs','rust']:
        video=json.loads((folder/f'{language}-video.json').read_text());verify(video['complete'])
        progress=[e for e in video['events'] if e['type']=='progress']
        assert len(progress)==60 and [e['result'] for e in progress]==expected
        assert [e['index'] for e in progress]==list(range(60))
    stats={}
    for language in ['robotjs','rust']:
        samples=[r['seconds']*1000 for r in manifest['runs'] if r['language']==language]
        stats[language]={'median_ms':statistics.median(samples),'min_ms':min(samples),'max_ms':max(samples)}
    ratio=stats['robotjs']['median_ms']/stats['rust']['median_ms']
    saved=stats['robotjs']['median_ms']-stats['rust']['median_ms']
    summary=dict(results=stats,ratio=ratio,saved_ms=saved,screenshots=60,targets=targets,verified_runs=12)
    write(folder/'summary.json',json.dumps(summary,indent=2)+'\n')
    base=f'workflows/2026-10-30-{case}/'
    saved_label=f'{saved/1000:.2f} s' if saved>=1000 else f'{saved:.2f} ms'
    parts.append(f'<section id="app-{case}"><h3>{title}</h3><p>{task}</p><div class="explorer" role="figure" aria-label="{title}: median analysis time"><p class="workload-result"><strong>{ratio:.2f}× faster</strong> · {saved_label} saved per 60-image batch</p><p class="chart-note">Milliseconds · shorter is faster</p>')
    for language,label,cls in [('robotjs','RobotJS','base'),('rust','Rust','rust')]:
        value=stats[language]['median_ms']
        parts.append(f'<div class="bar-line"><span>{label}</span><div class="bar-track" aria-hidden="true"><div class="bar-fill {cls}" style="width:{value/stats["robotjs"]["median_ms"]*100:.4f}%"></div></div><output>{value:,.2f} ms</output></div>')
    parts.append(f'''</div><figure class="demo-card"><video id="video-{case}" aria-label="{title}, RobotJS and Rust side by side" controls playsinline preload="none" poster="{base}workload.png"><source src="{base}workload.mp4" type="video/mp4"><a href="{base}workload.mp4">Download video</a></video><div class="links"><button type="button" data-video="video-{case}" data-rate="1">Replay at 1×</button><button type="button" data-video="video-{case}" data-rate="0.25">Inspect at ¼×</button></div><figcaption id="video-{case}-status" aria-live="polite">Playback: original speed (1×). Separate recorded takes; the finished side holds.</figcaption></figure><p>{reason}</p><details class="evidence"><summary>Test details, all six runs, and downloadable inputs</summary><p>Both use public find-all-color APIs on identical preloaded BGRA32 buffers, scan the same rectangle, and aggregate matching pixels by row into vertical regions. Warmup, input loading, and output checks are outside timing. Library order alternates over six rounds; no video is recorded during measurement.</p><p>The colors and scan area are fixed for these app layouts. This measures a color-based automation recipe, including API allocations and grouping loops. It does not compare custom JavaScript buffer scans or establish a general language speedup.</p><p>Expected rectangles come from an independent Python scan of the raw pixels, cross-checked against the app’s DOM geometry. GitHub’s collapsed table borders paint one pixel beyond some DOM rectangles; exact pixel-reference answers are retained alongside the original DOM coordinates. Every measured output must match that reference exactly.</p><div class="table-wrap" tabindex="0"><table><caption>Milliseconds per 60-image batch · all outputs verified</caption><thead><tr><th scope="col">Round</th><th scope="col">RobotJS</th><th scope="col">Rust</th></tr></thead><tbody>''')
    for n in range(1,7):
        rows={r['language']:r for r in manifest['runs'] if r['round']==n}
        parts.append(f'<tr><th scope="row">{n}</th>'+''.join(f'<td><a href="{base}{rows[l]["file"]}">{rows[l]["seconds"]*1000:.2f}</a></td>' for l in ['robotjs','rust'])+'</tr>')
    parts.append('</tbody></table></div><p>'+'; '.join(f'{label} range: {stats[l]["min_ms"]:.2f}–{stats[l]["max_ms"]:.2f} ms' for l,label in [('robotjs','RobotJS'),('rust','Rust')])+'.</p>')
    app_label='VS Code' if case=='vscode' else 'Chrome'
    parts.append(f'<p>{app_label} {capture["app_version"]}; Apple M6, macOS {manifest["os"]}, Node {manifest["node"]}, Rust release build. The completed capture corpus had no capture errors. An earlier discarded preparation attempt hit a ScreenCaptureKit timeout; capture reliability and latency are separate from these analysis results.</p><div class="links"><a href="{base}summary.json">Computed summary</a><a href="{base}manifest.json">Environment and run hashes</a><a href="{base}screenshots.zip">All 60 screenshots and raw pixels</a><a href="{base}fixtures.json">Expected rectangles and DOM geometry</a><a href="{base}capture.json">Native capture provenance</a><a href="https://github.com/priyankark/rusty-robot/tree/main/tools/benchmark-demo#real-application-screens">Source and reproduction</a></div></details></section>')
parts.append('</section>')
article=ROOT/'docs/building-rusty-robot.html'
source=article.read_text();begin='<!-- BEGIN REAL APP COMPARISONS -->';end='<!-- END REAL APP COMPARISONS -->'
assert source.count(begin)==source.count(end)==1
source=source[:source.index(begin)]+begin+'\n'+'\n'.join(parts)+'\n'+end+source[source.index(end)+len(end):]
write(article,source)
print('Verified both real-app corpora, independent pixel/DOM oracles, 24 measured batches, 4 video runs, and generated story.')
