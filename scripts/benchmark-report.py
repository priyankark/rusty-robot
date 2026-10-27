#!/usr/bin/env python3
"""Render the recorded benchmarks as a standalone, offline HTML report."""
import argparse
import csv
import json
import statistics
from pathlib import Path

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check', action='store_true', help='Fail if the checked-in report differs')
args = parser.parse_args()
source = root / 'benchmarks/2026-10-27'
ab = json.loads((source / 'optimization-ab.json').read_text())
current = json.loads((source / 'raw.json').read_text())
rows = []
for case in dict.fromkeys(r['case'] for r in ab['results']):
    values = {version: statistics.median([
        n for r in ab['results'] if r['case'] == case and r['version'] == version
        for n in r['ns_per_op']
    ]) / 1000 for version in ['before', 'after']}
    rows.append({'case': case, **values})
with (source / 'summary.csv').open() as f:
    comparison = [{'case': r['case'], 'before': float(r['robotjs_us']),
                   'after': float(r['rust_us'])} for r in csv.DictReader(f)]
payload = json.dumps({'ab': rows, 'js': comparison, 'metadata': current['metadata']}).replace('<', '\\u003c')
html = r'''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Rusty Robot vs RobotJS · Overall benchmark report</title>
<style>
:root{color-scheme:dark;--bg:#101417;--panel:#191f24;--ink:#edf2f4;--muted:#a7b4bd;--green:#8ce5ba;--orange:#ffc18a;--line:#303c44}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--ink);font:15px/1.6 system-ui,-apple-system,sans-serif}main{max-width:1140px;margin:auto;padding:54px 28px}header{border-bottom:1px solid var(--line);padding-bottom:28px}.eyebrow{color:var(--green);text-transform:uppercase;letter-spacing:.16em;font-size:12px;font-weight:700}h1{font-size:clamp(32px,5vw,52px);letter-spacing:-.04em;line-height:1.1;margin:18px 0}h2{font-size:21px;margin:0}p{color:var(--muted);max-width:850px}.cards{display:grid;grid-template-columns:repeat(4,1fr);gap:14px;margin:28px 0}.card{border:1px solid var(--line);background:var(--panel);border-radius:12px;padding:20px}.card strong{font-size:35px;display:block;color:var(--green);letter-spacing:-.04em}.card span{color:var(--muted);font-size:13px}.controls{display:flex;justify-content:space-between;align-items:center;gap:16px;flex-wrap:wrap;margin:34px 0 18px}.tabs{display:flex;gap:7px;flex-wrap:wrap}button,select{font:inherit;border:1px solid var(--line);background:var(--panel);color:var(--ink);padding:9px 13px;border-radius:7px;cursor:pointer}button[aria-pressed=true]{background:var(--green);color:#10271d;border-color:var(--green);font-weight:650}button:focus-visible,select:focus-visible{outline:3px solid var(--orange);outline-offset:3px}.legend{color:var(--muted);font-size:13px;margin:12px 0 20px}.dot{display:inline-block;width:10px;height:10px;border-radius:2px;margin-right:5px}.baseline{background:#778a99}.optimized{background:var(--green)}.table-wrap{overflow-x:auto;border:1px solid var(--line);border-radius:10px}table{width:100%;border-collapse:collapse;white-space:nowrap}th{text-align:left;color:var(--muted);font-size:12px;text-transform:uppercase;letter-spacing:.05em;background:var(--panel)}td,th{padding:13px 17px;border-bottom:1px solid var(--line)}tr:last-child td{border-bottom:0}td{font-variant-numeric:tabular-nums}td.num{text-align:right}th.num{text-align:right}.case{font-size:13px;font-family:ui-monospace,monospace}.bars{width:165px;display:grid;gap:5px}.bar{height:6px;min-width:1px;border-radius:2px}.win{color:var(--green);font-weight:650}.loss{color:var(--orange);font-weight:650}.notes{display:grid;grid-template-columns:1fr 1fr;gap:24px;margin-top:32px}.notes article{padding:22px;border:1px solid var(--line);border-radius:10px}.notes p{font-size:14px;margin-bottom:0}.warning{color:var(--orange)}details{margin-top:28px;border-top:1px solid var(--line);padding-top:18px}summary{cursor:pointer;color:var(--muted)}pre{overflow:auto;font-size:12px;color:var(--muted)}footer{color:var(--muted);font-size:12px;margin-top:30px}a{color:var(--green)}@media(max-width:700px){main{padding:28px 16px}.cards{grid-template-columns:1fr 1fr}.notes{grid-template-columns:1fr}.bars{width:100px}.card{padding:14px}.card strong{font-size:29px}}
</style></head><body><main>
<header><img src="../assets/robot.png" width="88" height="88" alt="Rusty Robot logo"><div class="eyebrow">Rusty Robot / Engineering benchmarks / 27 Oct 2026</div>
<h1>Rusty Robot vs RobotJS</h1>
<p><a href="index.html">New: real AirCodum and Agentum input/video benchmarks</a>. This page preserves the earlier API-only snapshot.</p>
<p>The complete measured comparison: bitmap access, color and image search, image codecs, cursor queries, display enumeration, and screen capture. Current optimized Rust versus RobotJS 0.9.1’s JavaScript API and native C/C++ addon, on an Apple M6 Mac.</p></header>
<section class="cards" aria-label="Overall Rust versus RobotJS measurements">
<div class="card"><strong id="total"></strong><span>Operations benchmarked</span></div>
<div class="card"><strong id="faster"></strong><span>Lower median time in Rust</span></div>
<div class="card"><strong id="slower" style="color:var(--orange)"></strong><span>Higher median time in Rust</span></div>
<div class="card"><strong id="capture-gap" style="color:var(--orange)"></strong><span>Single-pixel capture slowdown</span></div></section>
<p>Counts describe measured medians, not statistically established wins. Some round ranges overlap. There is no single overall speedup: your mix of operations determines end-to-end performance.</p>
<section class="notes" aria-label="Overall findings">
<article><h2>Bitmap access & search</h2><p>In-memory pixel reads are <b>15.65× faster</b>. Late and absent exact color searches are about <b>2.1× faster</b>. Exact image search is roughly tied; tolerant image search is <b>1.17× faster</b>. Returning 65,536 color matches is <b>267× faster</b>, including the difference between Rust vectors and JavaScript object allocation.</p></article>
<article><h2>Image loading & saving</h2><p>BMP loading takes Rust <b>1.13× as long</b>; PNG loading is <b>1.17× faster</b>. BMP saving is <b>1.19× faster by median</b>, with substantial RobotJS variation between rounds. PNG saving is essentially tied. BMP storage formats and PNG output sizes differ between APIs.</p></article>
<article><h2>Desktop queries & capture</h2><p>Cursor position queries are <b>3.85× faster</b>, but display enumeration takes <b>1.27× as long</b>. <span class="warning">Single-pixel and 100×100 captures take Rust 6.08× and 5.43× as long.</span> Full-screen capture is broadly comparable; overlapping round ranges make the 1.12× median advantage uncertain.</p></article>
<article><h2>What this report covers</h2><p>All <b>21 measured operations</b>, including unchanged paths and slowdowns. This is a performance comparison, not a complete feature-parity audit. Keyboard typing, mouse movement/click delivery, and end-to-end automation throughput were not benchmarked. Native desktop measurements cover macOS only.</p></article>
</section>
<div class="controls"><div class="tabs" aria-label="Comparison"><button id="js" aria-pressed="true">Overall: Rust vs RobotJS</button><button id="ab" aria-pressed="false">Optimization history</button></div><label>Sort <select id="sort"><option value="suite">Benchmark order</option><option value="speed">Largest speedup</option><option value="name">Operation name</option><option value="time">Optimized duration</option></select></label></div>
<h2 id="heading">All 21 operations: Rust versus RobotJS</h2><p id="description"></p>
<div class="legend"><span class="dot baseline"></span><span id="baseline-label">RobotJS</span> &nbsp; <span class="dot optimized"></span>Optimized Rust &nbsp; · &nbsp; Bars compare time within each row; shorter is faster.</div>
<div class="table-wrap"><table><thead><tr><th scope="col">Operation</th><th scope="col">Relative duration</th><th scope="col" class="num" id="before-label">RobotJS µs</th><th scope="col" class="num">Rust µs</th><th scope="col" class="num">Speedup</th></tr></thead><tbody id="results"></tbody></table></div>
<section class="notes"><article><h2>What changed</h2><p>Exact color searches use packed RGB comparisons, ignoring alpha. Image searches check needle pixels backwards. BMP loading copies compatible rows; saving assembles a single payload for bulk writing. Output formats and match ordering are preserved.</p></article>
<article><h2>Tradeoffs & remaining gaps</h2><p><span class="warning">Compared with the previous Rust build, tolerant image search measured about 2.7% slower; it remains faster than RobotJS.</span> BMP saving needs an extra output-sized buffer (6.22 MB here). Small screen captures remain substantially slower than RobotJS; their native backend was not changed.</p></article>
<article><h2>How to read the results</h2><p>Speedup = baseline time ÷ optimized Rust time. Above 1× favors optimized Rust. RobotJS uses a native C/C++ addon, not pure JavaScript. Exact image search is roughly tied with RobotJS. Dense result collection includes Vec versus JavaScript object allocation. BMP formats differ between APIs.</p></article>
<article><h2>Measurement & verification</h2><p>Three rounds × seven timed batches, warmed caches, release builds, sequential execution and alternating order. All 126 Rust/RobotJS and 78 Rust before/after process measurements passed. Headless tests and lint passed; 36 opt-in desktop tests were not rerun for these bitmap changes. At measurement time, native Linux/Windows verification was outstanding; see the current <a href="https://github.com/priyankark/rusty-robot/blob/main/docs/validation.md">validation record</a> for subsequent passes.</p></article></section>
<details><summary>Environment and reproducibility</summary><p>Timings are median batch averages, not tail latencies. Synthetic fixtures do not represent every screenshot. File writes exclude fsync. Rust BMP loads produce BGRA32 versus RobotJS BGR24; saves use BMP24 versus BMP32. RobotJS BMP saving varied from 1.080–2.181 ms across rounds. PNG and native desktop code were unchanged.</p><p>Regenerate measurements with <code>python3 scripts/benchmark.py</code>, then compare with this archived snapshot. Rebuild the archived report with <code>python3 scripts/benchmark-report.py</code>. Raw samples are checked in under <code>benchmarks/2026-10-27</code>. The historical before binary is not distributed; its hash is recorded. New measurements do not overwrite this published snapshot.</p><pre id="metadata"></pre></details>
<footer>Recorded measurements, not live telemetry. <a href="https://github.com/priyankark/rusty-robot">GitHub / MIT source</a> · <a href="https://github.com/priyankark/rusty-robot/tree/main/benchmarks/2026-10-27">Raw data</a> · <a href="https://priyankark.github.io/rusty-robot/api/rusty_robot/">API docs</a> · <a href="optimization-results.md">Detailed optimization record</a> · <a href="benchmarks.md">Original baseline</a></footer>
</main><script>
const data=__DATA__;
let mode='js';
const byId=id=>document.getElementById(id);
const format=n=>n.toLocaleString('en-US',{minimumFractionDigits:3,maximumFractionDigits:3});
byId('total').textContent=data.js.length;
byId('faster').textContent=data.js.filter(r=>r.after<r.before).length;
byId('slower').textContent=data.js.filter(r=>r.after>r.before).length;
const capture=data.js.find(r=>r.case==='get_pixel_color');
byId('capture-gap').textContent=(capture.after/capture.before).toFixed(2)+'×';
byId('metadata').textContent=JSON.stringify(data.metadata,null,2);
function render(){
 const rows=[...data[mode]];
 const order=byId('sort').value;
 if(order!=='suite')rows.sort(order==='name'?(a,b)=>a.case.localeCompare(b.case):order==='time'?(a,b)=>b.after-a.after:(a,b)=>b.before/b.after-a.before/a.after);
 byId('ab').setAttribute('aria-pressed',mode==='ab');byId('js').setAttribute('aria-pressed',mode==='js');
 byId('heading').textContent=mode==='ab'?'Before and after optimization':'All 21 operations: Rust versus RobotJS';
 byId('description').textContent=mode==='ab'?'13 operations against the saved pre-change Rust executable. The overall summary above continues to compare current Rust against RobotJS.':'All 21 operations against RobotJS 0.9.1 and its native addon. This is a separate measurement run from the paired Rust comparison.';
 byId('baseline-label').textContent=mode==='ab'?'Previous Rust':'RobotJS';
 byId('before-label').textContent=mode==='ab'?'Before µs':'RobotJS µs';
 byId('results').replaceChildren(...rows.map(r=>{
  const tr=document.createElement('tr'),max=Math.max(r.before,r.after),ratio=r.before/r.after;
  const name=document.createElement('td');name.className='case';name.textContent=r.case;tr.append(name);
  const graph=document.createElement('td'),bars=document.createElement('div');bars.className='bars';bars.setAttribute('aria-hidden','true');
  for(const [value,cls] of [[r.before,'baseline'],[r.after,'optimized']]){const bar=document.createElement('div');bar.className='bar '+cls;bar.style.width=(value/max*100)+'%';bars.append(bar);}graph.append(bars);tr.append(graph);
  for(const value of [r.before,r.after]){const td=document.createElement('td');td.className='num';td.textContent=format(value);tr.append(td);}
  const speed=document.createElement('td');speed.className='num '+(ratio>=1?'win':'loss');speed.textContent=ratio.toFixed(2)+'×';tr.append(speed);return tr;
 }));
}
for(const id of ['ab','js'])byId(id).addEventListener('click',()=>{mode=id;render()});byId('sort').addEventListener('change',render);render();
</script></body></html>'''
output = root / 'docs/benchmark-report.html'
rendered = html.replace('__DATA__', payload)
if args.check:
    if not output.exists() or output.read_text() != rendered:
        raise SystemExit('Report differs: run python3 scripts/benchmark-report.py')
else:
    output.write_text(rendered)
print(output)
