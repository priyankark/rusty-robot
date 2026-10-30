#!/usr/bin/env python3
"""Build the video gallery from the recorded demonstration results."""
from pathlib import Path
import argparse, hashlib, html, json
root=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check',action='store_true')
args=parser.parse_args()
folder=root/'docs/demos/2026-10-29'
manifest=json.loads((folder/'manifest.json').read_text())
parts=[]
for run in manifest['runs']:
    name=run['case'];record=json.loads((folder/(name+'.json')).read_text());c=record['config']
    assert hashlib.sha256((folder/(name+'.mp4')).read_bytes()).hexdigest()==run['video_sha256']
    results={r['language']:r for r in record['events'] if r['type']=='complete'}
    assert len(results)==2 and all(r['verified'] and r['done']==c['total'] for r in results.values())
    assert results['rust']['result']==results['robotjs']['result']
    base='demos/2026-10-29/'+name
    esc=html.escape
    parts.append(f'''<figure class="demo-card" id="{name}"><h2>{esc(c['title'])}</h2><p>{esc(c['subtitle'])}</p><video controls playsinline preload="none" poster="{base}.png" aria-label="{esc(c['title'])}: RobotJS on the left, Rusty Robot on the right" aria-describedby="{name}-caption"><source src="{base}.mp4" type="video/mp4">Your browser cannot play this video. <a href="{base}.mp4">Download the recording</a>.</video><div class="demo-results"><span>RobotJS: <strong>{results['robotjs']['seconds']:.3f} s</strong></span><span>Rust: <strong>{results['rust']['seconds']:.3f} s</strong></span><span>Both: <strong>PASS</strong></span></div><figcaption id="{name}-caption">{esc(c['result'])}. {esc(c['caveat'])} <a href="{base}.json">Recorded progress and results</a> · <a href="{base}.mp4" download>Download video</a></figcaption></figure>''')
page='''<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Watch Rusty Robot vs RobotJS · Side-by-side runs</title><meta name="description" content="Four real side-by-side API runs: color search, image search, and BMP loading. Videos show progress, verified results, and elapsed time."><link rel="stylesheet" href="site.css"><link rel="stylesheet" href="benchmark-explorer.css"></head><body><a class="skip" href="#main">Skip to videos</a><nav class="nav" aria-label="Main navigation"><a class="brand" href="index.html"><img src="assets/robot.png" alt="" width="42" height="42">Rusty Robot</a><div class="nav-links"><a href="building-rusty-robot.html">How we built it</a><a href="api-benchmarks.html">Benchmark graphs</a><a href="https://github.com/priyankark/rusty-robot">GitHub</a></div></nav><main id="main" class="wide"><div class="eyebrow">Real API calls · Verified answers · 1× playback</div><h1>Watch the two implementations run.</h1><p class="lede">RobotJS on the left. Rusty Robot on the right. Same input and operation count, with progress and final results visible.</p><p>Rusty Robot rewrites RobotJS-style desktop automation in Rust. These four recordings show image operations: each frame is a screen grab of a live progress window connected to the real Node/C++ and Rust processes.</p><nav class="toc" aria-label="Choose a video"><a href="#find_colors_dense">All pixel matches</a><a href="#find_color_absent">Missing color</a><a href="#find_image_late">Image match</a><a href="#load_bmp">Load BMP</a></nav>'''+''.join(parts)+'''<details><summary>How these videos were recorded</summary><p>The processes run concurrently on the same Apple M6 Mac. Each performs five warmups, waits for a shared start signal, and checks the answer on every operation. The clocks include assertions and progress reporting; setup is excluded. Both use the same saved input fixtures. Node uses RobotJS’s real native addon; Rust uses the 0.1.3 library source.</p><p>The recorder captures only this owned demo window at its original wall-clock pace. It does not replay the archived benchmark numbers or simulate the work. The input image stays visible while the actual API runs report progress. The videos have no audio; all results are also written above.</p><p>These are single-run demonstrations with recording and two processes sharing a machine. Use the <a href="api-benchmarks.html">sequential, repeated benchmark experiment</a> for the reported performance comparison. This does not measure mouse clicks, typing, or screenshot latency.</p><p>Recorded October 29 Pacific / October 30 UTC, 2026. <a href="demos/2026-10-29/manifest.json">Environment, executable hashes, and video checksums</a> · <a href="https://github.com/priyankark/rusty-robot/tree/main/tools/benchmark-demo">Recorder and runner source</a>.</p></details><footer class="footer">Wins and losses are included. Read the workload and output-format notes beside each video.</footer></main></body></html>'''
output=root/'docs/api-operation-demos.html'
if args.check:
    assert output.read_text()==page,'Run python3 scripts/demo-report.py'
else:output.write_text(page)
print('Verified four paired results and video hashes; '+str(output))
