#!/usr/bin/env python3
"""Publish actual desktop workflow recordings and verify their recorded outcomes."""
import argparse,hashlib,json
from pathlib import Path
root=Path(__file__).resolve().parents[1]
folder=root/'docs/workflows/2026-10-30'
manifest=json.loads((folder/'manifest.json').read_text())
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
assert {r['mode'] for r in manifest['runs']}=={'form','targets'}
results={}
for case in manifest['runs']:
    mode=case['mode'];assert hashlib.sha256((folder/(mode+'.mp4')).read_bytes()).hexdigest()==case['video_sha256']
    results[mode]={}
    for summary in case['results']:
        lang=summary['language'];run=json.loads((folder/(mode+'-'+lang+'.json')).read_text());state=run['state']
        assert run['seconds']==summary['seconds'] and run['verified']==summary['verified']
        assert state['active'] and not state['errors'] and state['misses']==0
        assert run['permissions']['accessibility'] and run['permissions']['capture']
        if mode=='form':
            assert run['verified'] and len(state['saved'])==8 and state['trusted']>0
            assert len([a for a in run['actions'] if a['command']['op']=='type'])==24
        elif lang=='rust':
            assert run['verified'] and state['hits']==48 and state['trusted']>0
            assert len([a for a in run['actions'] if a['command']['op']=='find'])==48
            assert len([a for a in run['actions'] if a['command']['op']=='click'])==48
        else:
            assert not run['verified'] and state['hits']==0
            assert state['failure']=='Failed to capture the requested screen rect.'
            assert run['actions'][-1]['error']==state['failure']
        results[mode][lang]=run
    if mode=='form':assert results[mode]['rust']['state']['saved']==results[mode]['robotjs']['state']['saved']
base='workflows/2026-10-30/'
def video(mode,title):
    return f'<video controls playsinline preload="none" poster="{base}{mode}.png" aria-label="{title}; RobotJS left, Rusty Robot right" aria-describedby="{mode}-caption"><source src="{base}{mode}.mp4" type="video/mp4"><a href="{base}{mode}.mp4">Download video</a></video>'
def raw(mode):
    return f'<a href="{base}{mode}-robotjs.json">RobotJS log</a> · <a href="{base}{mode}-rust.json">Rust log</a> · <a href="{base}{mode}.mp4" download>Download video</a>'
form=results['form'];targets=results['targets']
page='''<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rust vs RobotJS · Watch real desktop workflows</title><meta name="description" content="Watch native typing, form submission, and screenshot-driven clicks side by side. Real desktop recordings with verified results."><link rel="stylesheet" href="site.css"><link rel="stylesheet" href="benchmark-explorer.css"></head><body><a class="skip" href="#main">Skip to videos</a><nav class="nav" aria-label="Main navigation"><a class="brand" href="index.html"><img src="assets/robot.png" alt="" width="42" height="42">Rusty Robot</a><div class="nav-links"><a href="building-rusty-robot.html">How we built it</a><a href="api-benchmarks.html">Benchmark graphs</a><a href="https://github.com/priyankark/rusty-robot">GitHub</a></div></nav><main class="wide" id="main"><div class="eyebrow">Real desktop input · Original playback speed</div><h1>Earlier default-behavior recordings</h1><div class="note"><p>This archive uses different text-pacing behavior. It is not a controlled speed comparison. <a href="typing-comparison.html">See the corrected comparison with explicit settings and repeated runs</a>.</p></div><p class="lede">RobotJS on the left. Rusty Robot on the right. Actual typing, saved records, screenshots, and clicks in the same test app.</p><nav class="toc" aria-label="Choose a workflow"><a href="#form">Enter customer records</a><a href="#targets">Find and click moving tasks</a></nav>'''
page+=f'''<figure class="demo-card" id="form"><h2>Enter and save eight customer records</h2><p>Type a name, email, and follow-up note. Tab between fields and submit each record with Enter. Every saved field is checked against the source data.</p>{video('form','Enter eight customer records')}<div class="demo-results"><span>RobotJS: <strong>{form['robotjs']['seconds']:.3f} s</strong></span><span>Rust: <strong>{form['rust']['seconds']:.3f} s</strong></span><span>Both: <strong>8 / 8 verified</strong></span></div><figcaption id="form-caption">Both use their default 10 ms input-delay settings. RobotJS types per character; Rust batches text into native events. This single desktop run includes input delivery, app updates, verification, and recording. {raw('form')}</figcaption></figure>'''
page+=f'''<figure class="demo-card" id="targets"><h2>Find and click 48 changing targets</h2><p>Take a fresh screenshot of the board, search its pixels for the green task, and click the location found. The target moves after every successful click.</p>{video('targets','Find and click 48 moving tasks')}<div class="demo-results"><span>RobotJS: <strong>capture failed</strong></span><span>Rust: <strong>48 / 48 in {targets['rust']['seconds']:.3f} s</strong></span><span>Rust missed clicks: <strong>0</strong></span></div><figcaption id="targets-caption">RobotJS returned “Failed to capture the requested screen rect.” on this Mac, despite permission checks reporting access. Rust completed the task. This is the observed outcome of this run, not a speed ratio or a claim that RobotJS capture always fails. {raw('targets')}</figcaption></figure>'''
page+='''<details><summary>Recording and verification details</summary><p>Each implementation runs separately in the same owned Chrome test app on an Apple M6 Mac, macOS 27.0. The recordings are aligned at the task start and combined side by side at original speed; the finished pane holds its final frame. Synthetic customer data uses example.test addresses. There is no audio; results appear in the video and above.</p><p>Mouse and keyboard actions go through the libraries’ native OS APIs. The recorder reads app state to check that text and clicks arrived; it does not insert the form values or dispatch synthetic input. Input stops if the owned window loses focus. Target coordinates come from each library’s screen capture and color search, not the page’s target coordinates.</p><p>These are demonstrations of complete tasks, not repeated isolated latency measurements. The <a href="api-benchmarks.html">21-operation report</a> uses a separate, dated experiment. The capture failure here does not change those historical samples.</p><p><a href="workflows/2026-10-30/manifest.json">Environment, binary hashes, and checksums</a> · <a href="https://github.com/priyankark/rusty-robot/tree/main/tools/benchmark-demo">App, runners, and recorder source</a> · <a href="api-operation-demos.html">Earlier API batch recordings</a>.</p></details><footer class="footer">Rusty Robot implements desktop automation in Rust and calls the OS. RobotJS exposes its C/C++ implementation through Node.js. <a href="building-rusty-robot.html">Read how we built the port</a>.</footer></main></body></html>'''
output=root/'docs/default-workflow-demos.html'
if args.check:assert output.read_text()==page,'Run python3 scripts/workflow-report.py'
else:output.write_text(page)
print('Verified two real desktop workflows, matching saved records, 48 captured clicks, and recorded Node capture failure')
