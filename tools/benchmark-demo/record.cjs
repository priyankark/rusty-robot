// Node 24+, Chrome and ffmpeg. Records only the owned demo page, never the desktop.
const fs=require('node:fs'),path=require('node:path'),http=require('node:http');
const {spawn,execFileSync}=require('node:child_process');const {createInterface}=require('node:readline');const crypto=require('node:crypto');
const root=path.resolve(__dirname,'../..'),[robot,fixtures,outArg]=process.argv.slice(2);
if(!robot||!fixtures||!outArg)throw Error('Usage: node tools/benchmark-demo/record.cjs ROBOTJS FIXTURES OUTPUT');
const out=path.resolve(outArg);fs.mkdirSync(out,{recursive:true});
const cases=[
 {name:'find_colors_dense',total:1000,title:'Return every matching pixel',subtitle:'1,000 searches · the same 256 × 256 image · 65,536 positions returned per search',result:'65,536 matches per search',fixtureLabel:'Every pixel matches #334455',caveat:'Includes building the result collection: JavaScript objects in RobotJS; a Rust vector in Rusty Robot.'},
 {name:'find_color_absent',total:5000,title:'Search an image for a missing color',subtitle:'5,000 searches · the same 1920 × 1080 image · every search scans the full image',result:'no match, as expected',fixtureLabel:'Search for cyan #00ffff: absent',caveat:'No result collection: both implementations must establish that the requested color is absent.'},
 {name:'find_image_late',total:1500,title:'Find a small image near the bottom-right',subtitle:'1,500 searches · a 32 × 32 template inside the same 1920 × 1080 image',result:'match at (1856, 1016)',fixtureLabel:'The template is near the bottom-right corner',caveat:'Both return the same first match. Exact image-search timings were close in the archived benchmark.'},
 {name:'load_bmp',total:5000,title:'Load and decode the same BMP',subtitle:'5,000 loads · the same 1920 × 1080 image · warmed file cache',result:'1920 × 1080 + checked pixel',fixtureLabel:'Same BMP input file',caveat:'Output formats differ: RobotJS returns BGR24; Rusty Robot returns the larger BGRA32 buffer.'}
];
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const rust=path.join(__dirname,'target/release/rusty-robot-benchmark-demo');
let config,clients=[],trace=[],children=[],chrome,ws;
const sendEvent=x=>{trace.push(x);for(const res of clients)res.write('data: '+JSON.stringify(x)+'\n\n')};
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const server=http.createServer((req,res)=>{
 if(req.url==='/events'){res.writeHead(200,{'Content-Type':'text/event-stream','Cache-Control':'no-store'});res.write(': connected\n\n');clients.push(res);req.on('close',()=>clients=clients.filter(x=>x!==res));return}
 if(req.url==='/config'){res.setHeader('Content-Type','application/json');res.end(JSON.stringify(config));return}
 if(req.url==='/fixture'){res.setHeader('Content-Type','image/bmp');if(config.name==='find_colors_dense'){const w=256,h=256,b=Buffer.alloc(54+w*h*3);b.write('BM');b.writeUInt32LE(b.length,2);b.writeUInt32LE(54,10);b.writeUInt32LE(40,14);b.writeInt32LE(w,18);b.writeInt32LE(h,22);b.writeUInt16LE(1,26);b.writeUInt16LE(24,28);for(let i=54;i<b.length;i+=3){b[i]=85;b[i+1]=68;b[i+2]=51}res.end(b)}else res.end(fs.readFileSync(path.join(fixtures,'source.bmp')));return}
 res.setHeader('Content-Type','text/html');res.end(fs.readFileSync(path.join(__dirname,'stage.html')))
});
(async()=>{
 await new Promise(r=>server.listen(0,'127.0.0.1',r));const port=server.address().port;
 const tmp=fs.mkdtempSync('/tmp/rusty-video-');
 chrome=spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',['--headless=new','--disable-gpu','--no-first-run','--no-default-browser-check','--user-data-dir='+path.join(tmp,'profile'),'--remote-debugging-port=9438','about:blank'],{stdio:'ignore'});
 let tabs;for(let i=0;i<100;i++){try{tabs=await(await fetch('http://127.0.0.1:9438/json')).json();break}catch{await sleep(100)}}
 ws=new WebSocket(tabs.find(t=>t.type==='page'&&t.url==='about:blank').webSocketDebuggerUrl);await new Promise(r=>ws.addEventListener('open',r,{once:true}));
 let id=0;const pending=new Map();ws.addEventListener('message',e=>{const m=JSON.parse(e.data);if(pending.has(m.id)){const p=pending.get(m.id);pending.delete(m.id);m.error?p.reject(m.error):p.resolve(m.result)}});
 const send=(method,params={})=>new Promise((resolve,reject)=>{const n=++id;pending.set(n,{resolve,reject});ws.send(JSON.stringify({id:n,method,params}))});
 const evaluate=async expression=>(await send('Runtime.evaluate',{expression,returnByValue:true})).result.value;
 await send('Page.enable');await send('Emulation.setDeviceMetricsOverride',{width:1280,height:800,deviceScaleFactor:1,mobile:false});
 const metadata={date:new Date().toISOString(),commit:execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),cpu:execFileSync('sysctl',['-n','machdep.cpu.brand_string'],{encoding:'utf8'}).trim(),os:execFileSync('sw_vers',['-productVersion'],{encoding:'utf8'}).trim(),node:process.version,rustc:execFileSync('rustc',['--version'],{encoding:'utf8'}).trim(),robotjs_commit:execFileSync('git',['rev-parse','HEAD'],{cwd:robot,encoding:'utf8'}).trim(),sha256:{rust:sha(rust),robotjs:sha(path.join(robot,'build/Release/robotjs.node'))},fixtures:Object.fromEntries(['canvas.bgra','dense.bgra','needle.bgra','source.bmp'].map(n=>[n,sha(path.join(fixtures,n))])),method:'Concurrent live API demonstrations, 5 warmups, setup excluded. Checks and progress IPC included. 1x wall-time recording, not isolated benchmark samples.',runs:[]};
 for(const c of cases){
  config={...c,date:metadata.date.slice(0,10),cpu:metadata.cpu};trace=[];
  await send('Page.navigate',{url:`http://127.0.0.1:${port}/`});
  for(let i=0;i<100;i++){if(await evaluate('window.ready && [...document.images].every(i=>i.complete&&i.naturalWidth>0)')&&clients.length)break;await sleep(50)}
  if(!await evaluate('window.ready'))throw Error('Page not ready');
  const dir=path.join(tmp,c.name);fs.mkdirSync(dir);const frames=[];let recording=true;
  const recordStart=performance.now();const recordingTask=(async()=>{while(recording){const at=(performance.now()-recordStart)/1000;const shot=await send('Page.captureScreenshot',{format:'jpeg',quality:88});const file=path.join(dir,String(frames.length).padStart(5,'0')+'.jpg');fs.writeFileSync(file,Buffer.from(shot.data,'base64'));frames.push({at,file});await sleep(40)}})();
  children=[];
  const launches=[['robotjs','node',[path.join(__dirname,'robotjs.cjs'),path.resolve(robot),path.resolve(fixtures),c.name,String(c.total)]],['rust',rust,[path.resolve(fixtures),c.name,String(c.total)]]];
  const ready=[];const done=[];
  for(const [language,cmd,args] of launches){let readyResolve,readyReject;ready.push(new Promise((r,j)=>{readyResolve=r;readyReject=j}));
   const child=spawn(cmd,args,{stdio:['pipe','pipe','pipe']});children.push(child);let stderr='',completed=false;child.stderr.on('data',d=>stderr+=d);
   createInterface({input:child.stdout}).on('line',line=>{const x=JSON.parse(line);if(x.type==='ready')readyResolve();if(x.type==='complete')completed=true;sendEvent({...x,language})});
   done.push(new Promise((resolve,reject)=>{child.on('error',e=>{readyReject(e);reject(e)});child.on('exit',code=>{if(code===0&&completed)resolve();else{const error=Error(language+' failed: '+stderr);readyReject(error);reject(error)}})}));
  }
  await Promise.all(ready);await sleep(1200);sendEvent({type:'start'});for(const child of children)child.stdin.end('go\n');
  await Promise.all(done);if(await evaluate('window.demoError'))throw Error('Demo error');
  for(let i=0;i<40&&!await evaluate('window.complete');i++)await sleep(50);
  if(!await evaluate('window.complete'))throw Error('Results not shown');
  const final=trace.filter(x=>x.type==='complete');if(final.length!==2||final.some(x=>!x.verified||x.done!==c.total))throw Error('Incomplete results');
  const poster=await send('Page.captureScreenshot',{format:'png'});fs.writeFileSync(path.join(out,c.name+'.png'),Buffer.from(poster.data,'base64'));
  await sleep(2500);recording=false;await recordingTask;
  const concat=frames.map((f,i)=>`file '${f.file}'\nduration ${i+1<frames.length?frames[i+1].at-f.at:0.05}\n`).join('')+`file '${frames.at(-1).file}'\n`;
  fs.writeFileSync(path.join(dir,'frames.txt'),concat);
  execFileSync('ffmpeg',['-hide_banner','-loglevel','error','-y','-f','concat','-safe','0','-i',path.join(dir,'frames.txt'),'-vf','fps=25','-c:v','libx264','-crf','22','-pix_fmt','yuv420p','-movflags','+faststart','-an',path.join(out,c.name+'.mp4')]);
  fs.writeFileSync(path.join(out,c.name+'.json'),JSON.stringify({config,events:trace},null,2)+'\n');metadata.runs.push({case:c.name,results:final,frames:frames.length,video_seconds:frames.at(-1).at,video_sha256:sha(path.join(out,c.name+'.mp4'))});console.log(c.name,JSON.stringify(final));
 }
 fs.writeFileSync(path.join(out,'manifest.json'),JSON.stringify(metadata,null,2)+'\n');await send('Browser.close');server.close();
})().catch(e=>{console.error(e);for(const c of children)c.kill();if(chrome)chrome.kill();if(ws)ws.close();server.close();process.exitCode=1});
