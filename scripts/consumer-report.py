#!/usr/bin/env python3
"""Generate the app/processing report from recorded raw measurements; --check verifies it."""
import argparse, json, statistics, math, html
from pathlib import Path
root=Path(__file__).resolve().parents[1]
source=root/'benchmarks/2026-10-27-consumers'
parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args()
def median(xs):return statistics.median(xs)
def percentile(xs,p):return sorted(xs)[max(0,math.ceil(len(xs)*p)-1)]
air=json.loads((source/'aircodum.json').read_text())
agent=json.loads((source/'agentum-stream.json').read_text())
inputs=json.loads((source/'agentum-input.json').read_text())
processing=json.loads((source/'processing/raw.json').read_text())
summary={'stream':[],'input':[],'processing':[],'metadata':json.loads((source/'manifest.json').read_text())}
for app,data in [('AirCodum',air),('Agentum',agent)]:
 assert len(data['runs'])==6
 for workload in ['static','motion']:
  for backend in ['robotjs','rusty-robot']:
   runs=[next(v for v in r['video']['results'] if v['workload']==workload) for r in data['runs'] if r['backend']==backend]
   assert len(runs)==3
   for run in runs:
    assert run['correctness']['jpeg_decoded'] and run['correctness']['dimensions']
    assert all(f['width']==1440 and f['height']==810 for f in run['frames'])
   frames=[f for r in runs for f in r['frames']];pings=[p for r in runs for p in r['ping_rtt_ms']]
   gaps=[b['received_ms']-a['received_ms'] for r in runs for a,b in zip(r['frames'],r['frames'][1:])]
   summary['stream'].append(dict(app=app,workload=workload,backend=backend,fps=median([r['delivered_fps'] for r in runs]),
    fps_range=[min(r['delivered_fps'] for r in runs),max(r['delivered_fps'] for r in runs)],
    first_ms=median([r['first_frame_ms'] for r in runs]),ping_p95_ms=percentile(pings,.95),gap_p95_ms=percentile(gaps,.95),
    bytes=median([f['bytes'] for f in frames]),host_cpu_percent=median([r['host_cpu_percent'] for r in runs]),
    loop_p95_ms=median([r['event_loop_p95_ms'] for r in runs]),count=len(frames),runs=3))
for app,data in [('AirCodum',air),('Agentum',inputs)]:
 assert len(data['runs'])==6
 names=list(dict.fromkeys(row['case'] for r in data['runs'] for row in r['results']))
 for name in names:
  item={'app':app,'case':name}
  for backend in ['robotjs','rusty-robot']:
   rows=[row for r in data['runs'] if r['backend']==backend for row in r['results'] if row['case']==name]
   assert len(rows)==15
   passed=[row['delivered_ms'] for row in rows if row['passed']]
   assert all(row['delivered_ms'] is None for row in rows if not row['passed'])
   item[backend]={'passed':len(passed),'total':len(rows),'median_ms':median(passed) if passed else None,'p95_ms':percentile(passed,.95) if passed else None}
  summary['input'].append(item)
for name in ['decode_encode','decode_resize_encode']:
 item={'case':name}
 for backend in ['robotjs','rusty-robot']:
  runs=[next(c for c in r['cases'] if c['name']==name) for r in processing['runs'] if r['backend']==backend]
  assert len(runs)==3
  samples=[s for r in runs for s in r['samples']];assert len(samples)==45
  item[backend]={'median_ms':median([s['ms'] for s in samples]),'p95_ms':percentile([s['ms'] for s in samples],.95),
   'cpu_ms':median([s['cpu_ms'] for s in samples]),'bytes':median([s['bytes'] for s in samples]),'psnr_db':median([r['psnr_db'] for r in runs])}
 summary['processing'].append(item)
payload=json.dumps(summary,separators=(',',':')).replace('<','\\u003c')
page='''<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Rusty Robot · Real app benchmarks</title><style>
:root{color-scheme:dark;--bg:#101619;--panel:#192327;--text:#eef4f1;--muted:#a9bbb7;--line:#34443e;--green:#8be6b6;--copper:#ffbc87}*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--text);font:15px/1.6 system-ui,sans-serif}main{max-width:1190px;margin:auto;padding:40px 24px}h1{font-size:clamp(32px,5vw,55px);line-height:1.1;letter-spacing:-.04em}h2{font-size:23px}p{color:var(--muted);max-width:940px}a{color:var(--green)}header{border-bottom:1px solid var(--line);padding-bottom:24px}.eyebrow{color:var(--copper);text-transform:uppercase;letter-spacing:.15em;font-size:12px}.cards{display:grid;grid-template-columns:repeat(3,1fr);gap:14px;margin:26px 0}.card{background:var(--panel);border:1px solid var(--line);border-radius:12px;padding:20px}.card strong{display:block;color:var(--green);font-size:34px}.card span{color:var(--muted);font-size:13px}button,select{background:var(--panel);color:var(--text);border:1px solid var(--line);border-radius:6px;padding:9px 14px;font:inherit;cursor:pointer}button[aria-pressed=true]{background:var(--green);color:#10251a}button:focus-visible,select:focus-visible{outline:3px solid var(--copper)}nav{display:flex;gap:10px;flex-wrap:wrap;margin:24px 0}.wrap{overflow:auto;border:1px solid var(--line);border-radius:9px;margin:20px 0}table{width:100%;border-collapse:collapse;white-space:nowrap}th,td{padding:12px 15px;text-align:right;border-bottom:1px solid var(--line);font-variant-numeric:tabular-nums}th{font-size:12px;color:var(--muted);background:var(--panel)}th:first-child,td:first-child{text-align:left}.good{color:var(--green)}.warn{color:var(--copper)}details{border-top:1px solid var(--line);padding:18px 0}summary{cursor:pointer}pre{overflow:auto;font-size:12px}footer{color:var(--muted);margin-top:24px;font-size:13px}small{color:var(--muted)}@media(max-width:650px){main{padding:24px 14px}.cards{grid-template-columns:1fr}.card{padding:12px}.card strong{font-size:28px}}
</style></head><body><main><header><img src="../assets/robot.png" width="88" height="88" alt="Rusty Robot mascot"><div class="eyebrow">Engineering evidence · macOS · 27 October 2026</div><h1>Real apps. Real frames.<br>Measured against the original.</h1><p>AirCodum inside an actual VS Code extension host, and Agentum's actual VNC server. Rust handles input, capture, resize, frame hashing and JPEG/PNG encoding through bounded workers. The original app paths use RobotJS for input and native macOS subprocess capture/resize for video.</p><p><a href="api-benchmarks.html">21-operation RobotJS API comparison</a> · <a href="https://github.com/priyankark/rusty-robot">Source &amp; agent skill</a> · <a href="benchmarks/2026-10-27-consumers/README.md">Method and raw evidence</a></p></header><section class="cards" id="cards"></section>
<p>Three independent runs per backend, alternating order. Video: 1440×810, JPEG quality 85, target 30 FPS, adaptation disabled, ten seconds each of owned static and 30 Hz motion content. Results describe this Mac and loopback transport; they are not an internet/mobile latency claim. Nominal quality numbers do not guarantee identical encoder fidelity.</p>
<nav aria-label="Benchmark view"><button data-tab="stream" aria-pressed="true">Live video</button><button data-tab="input" aria-pressed="false">Verified input</button><button data-tab="processing" aria-pressed="false">Image processing</button><label>App <select id="app"><option>AirCodum</option><option>Agentum</option></select></label></nav><h2 id="title"></h2><p id="description"></p><div class="wrap"><table><thead id="head"></thead><tbody id="body"></tbody></table></div><p id="note"></p>
<details><summary>Measurement boundaries and remaining work</summary><p>Video measures capture through resize/encode, authenticated WebSocket transport and receipt of JPEG bytes. The first and last frames are decoded after timing; every frame's dimensions and motion changes are checked. It does not measure rendering on a remote phone. Ping RTT measures control responsiveness while video runs.</p><p>CPU is the Node/extension-host process, including Rust worker threads and benchmark bookkeeping. It excludes the original path's screencapture/sips subprocesses, WindowServer, VS Code renderer and stimulus. These CPU numbers cannot establish total energy or system-CPU savings.</p><p>Static frame rate is intentionally low because unchanged frames are suppressed. Motion delivered FPS and inter-frame gaps are the throughput metrics. JPEG file sizes and quality differ by encoder. No composite “overall speedup” mixes unrelated operations.</p><p>Rust's one-shot ScreenCaptureKit capture still limits sustained video below the requested 30 FPS. A persistent capture stream, dirty-region transport and remote-display latency tests remain future work. This is a validated preview, not a finished VNC product.</p><p>Agentum input measurements isolate input by stubbing the unrelated capture subscription. Its video measurements use the real capture and server. AirCodum input runs through real VS Code; video is stopped for isolated input measurements. Failed original Unicode/multiline deliveries are correctness failures and receive no speedup ratio.</p></details><details><summary>Metadata and round ranges</summary><pre id="metadata"></pre></details><footer>Raw samples and source revisions accompany the report. Earlier slow per-character input measurements are preserved as a separate checkpoint. CI verifies report consistency.</footer></main><script>
const data=PAYLOAD;const f=(v,d=2)=>v==null?'—':v.toFixed(d);const backend=b=>b==='rusty-robot'?'Rust':'Original';
const motion=data.stream.filter(r=>r.workload==='motion');document.querySelector('#cards').innerHTML=['AirCodum','Agentum'].map(app=>{const o=motion.find(r=>r.app===app&&r.backend==='robotjs'),r=motion.find(r=>r.app===app&&r.backend==='rusty-robot');return `<div class="card"><strong>${f(r.fps/o.fps)}×</strong><span>${app} motion FPS: ${f(o.fps,1)} → ${f(r.fps,1)} · median of 3 runs</span></div>`}).join('')+`<div class="card"><strong>${data.input.reduce((a,r)=>a+r['rusty-robot'].passed,0)}/${data.input.reduce((a,r)=>a+r['rusty-robot'].total,0)}</strong><span>Rust input deliveries correct across both apps</span></div>`;
let tab='stream';const app=document.querySelector('#app');function render(){app.disabled=tab==='processing';const selected=app.value;let headers,rows,title,description,note;
if(tab==='stream'){title=selected+' · full video delivery';description='Higher motion FPS is better. Lower first-frame latency and ping RTT are better. Static FPS reflects suppression, not maximum throughput.';headers=['Workload / backend','FPS median [round range]','First frame ms','Frame-gap p95 ms','Ping p95 ms','Median KiB','Host CPU %'];rows=data.stream.filter(r=>r.app===selected).map(r=>[r.workload+' / '+backend(r.backend),f(r.fps)+' ['+r.fps_range.map(v=>f(v,1)).join('–')+']',f(r.first_ms),f(r.gap_p95_ms),f(r.ping_p95_ms),f(r.bytes/1024,1),f(r.host_cpu_percent,1)]);note='CPU excludes original capture/resize subprocesses. One fully busy CPU core is 100%. The shared Rust resize pool has four threads; the media queue admits at most two jobs.';}
else if(tab==='input'){title=selected+' · socket-to-editor correctness and latency';description='15 measured repetitions per case, across 3 processes, after warm-up. Time ends when the owned native target contains the expected text/state; polling resolution is 2 ms.';headers=['Operation','Original passed','Rust passed','Original median ms','Rust median ms','Rust p95 ms','Valid speedup'];rows=data.input.filter(r=>r.app===selected).map(r=>{const o=r.robotjs,n=r['rusty-robot'];return[r.case,o.passed+'/'+o.total,n.passed+'/'+n.total,f(o.median_ms),f(n.median_ms),f(n.p95_ms),o.passed===o.total&&n.passed===n.total?f(o.median_ms/n.median_ms)+'×':'Invalid original output'];});note='Fast return is not successful delivery. Original Unicode corruption and dropped line breaks are not counted as faster results. Mouse and shortcut timing includes configured native event delays.';}
else{title='Image processing · same owned PNG input';description='Decode → optional bilinear resize → JPEG quality 85. 45 samples per backend per case, 3 processes, 1 warm-up. Original here means Jimp, the app image-processing dependency; RobotJS itself does not JPEG-encode.';headers=['Case / backend','Median ms','p95 ms','CPU ms','KiB','PSNR dB','Speedup'];rows=data.processing.flatMap(r=>['robotjs','rusty-robot'].map(b=>{const v=r[b];return[r.case+' / '+(b==='robotjs'?'Jimp':'Rust'),f(v.median_ms),f(v.p95_ms),f(v.cpu_ms),f(v.bytes/1024,1),f(v.psnr_db),b==='rusty-robot'?f(r.robotjs.median_ms/v.median_ms)+'×':'—'];}));note='PSNR is against an independently computed area-average RGB reference. Higher is better. Different resamplers and JPEG encoders can trade fidelity for size/speed; inspect these columns together. This is not a desktop-capture benchmark.';}
document.querySelector('#title').textContent=title;document.querySelector('#description').textContent=description;document.querySelector('#note').textContent=note;document.querySelector('#head').innerHTML='<tr>'+headers.map(h=>'<th scope="col">'+h+'</th>').join('')+'</tr>';document.querySelector('#body').innerHTML=rows.map(r=>'<tr>'+r.map(v=>'<td>'+v+'</td>').join('')+'</tr>').join('');}
for(const b of document.querySelectorAll('button[data-tab]'))b.onclick=()=>{tab=b.dataset.tab;document.querySelectorAll('button[data-tab]').forEach(x=>x.setAttribute('aria-pressed',x===b));render();};app.onchange=render;document.querySelector('#metadata').textContent=JSON.stringify({metadata:data.metadata,rounds:data.stream.map(r=>({app:r.app,workload:r.workload,backend:r.backend,fps_range:r.fps_range}))},null,2);render();
</script></body></html>'''.replace('PAYLOAD',payload)
outputs={root/'docs/consumer-benchmarks.html':page,source/'summary.json':json.dumps(summary,indent=2)+'\n'}
for path,text in outputs.items():
 if args.check:assert path.read_text()==text,f'Stale report: {path}'
 else:path.write_text(text)
print('Verified consumer evidence and report' if args.check else 'Rendered consumer report')
