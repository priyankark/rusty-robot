#!/usr/bin/env python3
"""Verify and explain repeated desktop comparisons with explicit input settings."""
import argparse,hashlib,json,statistics
from pathlib import Path
root=Path(__file__).resolve().parents[1]
folder=root/'docs/workflows/2026-10-30-controlled'
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
manifest=json.loads((folder/'manifest.json').read_text())
assert len(manifest['runs'])==28 and len(manifest['videos'])==2
profiles={p['id']:p for p in manifest['profiles']}
reference=None
for summary in manifest['runs']:
    run=json.loads((folder/summary['file']).read_text())
    assert run['verified']==summary['verified'] and run['seconds']==summary['seconds']
    assert not run['recorded']
    p=profiles[run['profile']]
    assert p['mouse_ms']==10 and p['keyboard_ms']==0
    if run['verified']:
        types=[a['command'] for a in run['actions'] if a['command']['op']=='type']
        assert len(types)==24 and all(c.get('cpm')==p.get('cpm') for c in types)
        requested=[[c['text'] for c in types[i:i+3]] for i in range(0,24,3)]
        assert run['state']['saved']==requested and len(requested)==8
        assert run['state']['trusted']>0 and run['state']['active'] and not run['state']['errors']
        if reference is None:reference=requested
        assert requested==reference
for profile in profiles:
    ordered=[r for r in manifest['runs'] if r['profile']==profile and not r['warmup']]
    assert len(ordered)==12
    for round_id in range(1,7):
        pair=[r for r in ordered if r['round']==round_id]
        assert [r['language'] for r in pair]==(['robotjs','rust'] if round_id%2 else ['rust','robotjs'])
for video in manifest['videos']:
    assert hashlib.sha256((folder/(video['profile']+'.mp4')).read_bytes()).hexdigest()==video['sha256']
    for summary in video['results']:
        run=json.loads((folder/summary['file']).read_text())
        assert run['recorded'] and run['verified']==summary['verified'] and run['seconds']==summary['seconds']
        if run['verified']:assert run['state']['saved']==reference
stats={}
for profile in profiles:
    stats[profile]={}
    for lang in ['robotjs','rust']:
        rows=[r for r in manifest['runs'] if r['profile']==profile and r['language']==lang and not r['warmup']]
        samples=[r['seconds'] for r in rows if r['verified']]
        stats[profile][lang]={'passed':len(samples),'attempts':len(rows),'median':statistics.median(samples) if samples else None,'min':min(samples) if samples else None,'max':max(samples) if samples else None,'samples':samples}
summary=json.dumps({'profiles':stats,'typed_characters':sum(len(v) for row in reference for v in row)},indent=2)+'\n'
base='workflows/2026-10-30-controlled/'
page='''<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rust vs RobotJS · A controlled desktop comparison</title><meta name="description" content="Same form, same data, explicit input settings. Six alternating runs per library, exact field verification, and separate side-by-side recordings."><link rel="stylesheet" href="site.css"><link rel="stylesheet" href="benchmark-explorer.css"></head><body><a class="skip" href="#main">Skip to results</a><nav class="nav" aria-label="Main navigation"><a class="brand" href="index.html"><img src="assets/robot.png" alt="" width="42" height="42">Rusty Robot</a><div class="nav-links"><a href="building-rusty-robot.html">How we built it</a><a href="api-benchmarks.html">API benchmarks</a><a href="https://github.com/priyankark/rusty-robot">GitHub</a></div></nav><main class="wide" id="main"><div class="eyebrow">Same task · Explicit settings · Repeated runs</div><h1>Same form. Controlled settings.</h1><p class="lede">Enter and save eight customer records: name, email, and a follow-up note. Compare RobotJS and Rusty Robot after accounting for how they pace typing.</p><div class="reading-guide"><p><strong>Correction:</strong> the earlier default-settings video showed a large Rust advantage because RobotJS waits between characters while Rust batches text. That was not an apples-to-apples typing-rate comparison. The results below use explicit settings and repeated runs.</p><p>Both libraries receive the same data and must save every field correctly. Each condition has one warmup and six measured runs per library, with execution order reversed every round. Video capture is off during measurements.</p></div>'''
for profile,title,description in [('zero','1. Request zero keyboard delay','Both APIs use keyboard delay 0 ms and mouse delay 10 ms. This compares their normal text-entry APIs with configurable keyboard pacing disabled. Rust still has an internal 10 ms safety floor between text batches; RobotJS sends per-character events. The implementations are different, but the requested task, settings, and required result match.'),('paced','2. Match the character rate','Both use their explicit paced-typing API at 6,000 characters per minute (100 per second). Mouse delay is 10 ms; key-tap delay is 0 ms. This removes the default per-character versus per-batch pacing difference.')]:
    page+=f'<section id="{profile}"><h2>{title}</h2><p>{description}</p><div class="table-wrap" tabindex="0"><table><caption>Complete task time in seconds. Median and observed range across six runs; lower is better.</caption><thead><tr><th scope="col">Library</th><th scope="col">Median</th><th scope="col">Range</th><th scope="col">Verified</th></tr></thead><tbody>'
    for lang,label in [('robotjs','RobotJS / Node'),('rust','Rusty Robot')]:
        x=stats[profile][lang];median=f"{x['median']:.3f} s" if x['median'] is not None else 'No successful run';span=f"{x['min']:.3f}–{x['max']:.3f} s" if x['min'] is not None else '—'
        page+=f'<tr><th scope="row">{label}</th><td>{median}</td><td>{span}</td><td>{x["passed"]} / {x["attempts"]}</td></tr>'
    page+='</tbody></table></div>'
    video=next(v for v in manifest['videos'] if v['profile']==profile)
    page+=f'<figure class="demo-card"><video controls playsinline preload="none" poster="{base}{profile}.png" aria-label="{title}: RobotJS left, Rusty Robot right" aria-describedby="{profile}-caption"><source src="{base}{profile}.mp4" type="video/mp4"><a href="{base}{profile}.mp4">Download video</a></video><figcaption id="{profile}-caption">A separate illustrative run recorded after the measurements. Both panes play at original speed and are aligned at task start. The completed pane holds its final frame. Video times are not the medians above. <a href="{base}{profile}.mp4" download>Download video</a></figcaption></figure></section>'
page+='''<h2>What this tells us</h2><p>The earlier default-settings gap does not survive controlled pacing. These results describe the complete form task on one Mac, including OS delivery, app updates, and verification. They do not isolate language execution speed or establish a general winner for desktop automation.</p><p>The app is a purpose-built browser test fixture, using synthetic customer data. Mouse and keyboard events go through the real native APIs. The harness reads the app to check delivery; it does not inject form values or click events through JavaScript. The run stops if the owned window loses focus.</p><p>We could not establish a working RobotJS capture baseline in this session. The earlier capture failure is excluded from the speed comparison. <a href="workflows/2026-10-30/targets-robotjs.json">The failure log remains available</a>.</p><details><summary>Method, source, and all runs</summary><p>Apple M6, macOS 27.0, Node 24.22.0. RobotJS is pinned to the recorded source revision; Rusty Robot uses 0.1.3 library code. Each run starts with a fresh driver process and a reset app. Startup/reset are outside the timer. The timer begins before the initial native click and ends when the app has received and verified the eighth submitted record. Read-only checks and local control IPC are included.</p><p>The paced condition controls the requested character rate; native event delivery and scheduling still add overhead. Six trials and their ranges are descriptive, not confidence intervals. The recorded illustrations are additional runs, not selected fastest samples. Videos are silent; text results appear in the page.</p><div class="links"><a href="workflows/2026-10-30-controlled/summary.json">Computed summary and samples</a><a href="workflows/2026-10-30-controlled/manifest.json">All run references, settings, and hashes</a><a href="https://github.com/priyankark/rusty-robot/tree/main/tools/benchmark-demo">App and runner source</a><a href="default-workflow-demos.html">Earlier default-behavior recordings</a></div></details><footer class="footer">Recorded October 30 UTC / October 29 Pacific, 2026. Rusty Robot is a Rust implementation of RobotJS-style desktop automation; it does not wrap RobotJS’s C/C++ addon.</footer></main></body></html>'''
for p,text in [(folder/'summary.json',summary),(root/'docs/typing-comparison.html',page)]:
    if args.check:assert p.read_text()==text,f'Run python3 scripts/controlled-workflow-report.py ({p.name})'
    else:p.write_text(text)
print(summary)
