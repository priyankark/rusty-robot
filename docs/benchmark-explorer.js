(() => {
'use strict';
const data=window.RUSTY_BENCHMARKS;
const element=(tag,cls,text)=>{const e=document.createElement(tag);if(cls)e.className=cls;if(text!==undefined)e.textContent=text;return e};
const duration=n=>n>=1000?(n/1000).toFixed(3)+' ms':n.toFixed(3)+' µs';
for(const [index,host] of [...document.querySelectorAll('[data-benchmark-explorer]')].entries()){
 host.classList.add('explorer');host.replaceChildren();
 const controls=element('div','explorer-controls');
 const select=(label,options,value)=>{const wrap=element('label',null,label),s=element('select');for(const [val,text] of options){const o=element('option',null,text);o.value=val;s.append(o)}s.value=value;wrap.append(s);controls.append(wrap);return s};
 const mode=select('Compare',[['js','Rust vs RobotJS / Node'],['ab','Rust before vs after']],'js');
 const family=select('Workload',[['all','All operations'],['color','Pixels & colors'],['image','Image matching'],['files','Image files'],['desktop','Desktop & screenshots']],host.dataset.family||'all');
 const sort=select('Order',[['suite','Original order'],['speed','Largest ratio first'],['time','Longest Rust time first']],'suite');
 const note=element('p','chart-note');const key=element('div','chart-key');const baseKey=element('span');baseKey.append(element('i','swatch base'),document.createTextNode('RobotJS'));const rustKey=element('span');rustKey.append(element('i','swatch rust'),document.createTextNode('Rust'));key.append(baseKey,rustKey);
 const guide=element('p','chart-note','Shorter bars mean less time. Each pair has its own scale, starting at zero. Hover, focus, or tap a case name for details. 1 ms = 1,000 µs.');
 const count=element('p','explorer-count');count.setAttribute('aria-live','polite');
 const chart=element('div');const exact=element('details');exact.append(element('summary',null,'Exact values in microseconds (µs)'));const tableWrap=element('div','table-wrap');tableWrap.tabIndex=0;const table=element('table');tableWrap.append(table);exact.append(tableWrap);
 host.append(controls,note,key,guide,count,chart,exact);
 function render(){
  const baseline=mode.value==='js'?'RobotJS':'Before Rust';
  baseKey.lastChild.textContent=baseline;rustKey.lastChild.textContent=mode.value==='js'?'Rust':'After Rust';
  note.textContent=mode.value==='js'?'October 27, 2026 · Apple M6 / macOS · Node API + C/C++ addon versus Rust API + Rust implementation. Capture results predate the later Rust update.':'October 27, 2026 · Separate paired run of the old and optimized Rust executables. This view measures our changes, not Node versus Rust.';
  let rows=data[mode.value].filter(r=>family.value==='all'||data.cases[r.case].group===family.value);
  if(sort.value==='speed')rows.sort((a,b)=>b.before/b.after-a.before/a.after);if(sort.value==='time')rows.sort((a,b)=>b.after-a.after);
  count.textContent=rows.length+' operations shown · medians of timed batch averages';chart.replaceChildren();table.replaceChildren();
  const head=element('thead'),hr=element('tr');for(const x of ['Operation',baseline+' µs','Rust µs','Baseline / Rust']){const h=element('th',null,x);h.scope='col';hr.append(h)}head.append(hr);table.append(head);const body=element('tbody');table.append(body);
  for(const r of rows){
   const c=data.cases[r.case],ratio=r.before/r.after,max=Math.max(r.before,r.after),row=element('section','chart-row');
   const heading=element('div','chart-heading'),button=element('button','case-help',c.label+' ');button.type='button';button.append(element('span',null,'ⓘ'));
   const detail=element('div','case-detail');detail.id='case-'+index+'-'+r.case;detail.hidden=true;detail.append(element('p',null,c.description),element('p',null,c.caveat),element('code',null,r.case));
   button.setAttribute('aria-expanded','false');button.setAttribute('aria-controls',detail.id);
   let pinned=false;const show=()=>{detail.hidden=false;button.setAttribute('aria-expanded','true')},hide=()=>{detail.hidden=true;button.setAttribute('aria-expanded','false')};
   button.addEventListener('mouseenter',show);button.addEventListener('mouseleave',()=>{if(!pinned&&document.activeElement!==button)hide()});button.addEventListener('focus',show);button.addEventListener('blur',()=>{if(!pinned)hide()});button.addEventListener('click',()=>{pinned=!pinned;pinned?show():hide()});button.addEventListener('keydown',e=>{if(e.key==='Escape'){pinned=false;hide();e.preventDefault()}});
   const outcome=element('span','chart-outcome'+(ratio<1?' slower':''),ratio>=1?ratio.toFixed(2)+'× faster in Rust (median)':(1/ratio).toFixed(2)+'× longer in Rust');
   heading.append(button,outcome);row.append(heading,detail);
   for(const [label,value,cls] of [[baseline,r.before,'base'],[mode.value==='js'?'Rust':'After Rust',r.after,'rust']]){const line=element('div','bar-line'),track=element('div','bar-track'),bar=element('div','bar-fill '+cls);track.setAttribute('aria-hidden','true');bar.style.width=(value/max*100)+'%';track.append(bar);line.append(element('span',null,label),track,element('output',null,duration(value)));row.append(line)}
   if(mode.value==='js'&&['find_image_late','find_image_absent','save_png','capture_full'].includes(r.case))row.append(element('p','chart-note','Round ranges overlap: this small median difference is not a clear win.'));
   chart.append(row);const tr=element('tr');const th=element('th',null,c.label);th.scope='row';tr.append(th);for(const x of [r.before.toFixed(3),r.after.toFixed(3),ratio.toFixed(2)+'×'])tr.append(element('td',null,x));body.append(tr);
  }
 }
 for(const control of [mode,family,sort])control.addEventListener('change',render);render();
}
})();
