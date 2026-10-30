// Records actual OS input into an owned Chrome app. Read-only DOM assertions verify delivery.
const fs=require('node:fs'),path=require('node:path'),http=require('node:http'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process');const {createInterface}=require('node:readline');
const root=path.resolve(__dirname,'../..'),[robotArg,outArg]=process.argv.slice(2);
if(!robotArg||!outArg)throw Error('Usage: node workflow-compare.cjs ROBOTJS NEW_OUTPUT_DIRECTORY');
const robot=path.resolve(robotArg),out=path.resolve(outArg);if(fs.existsSync(out)&&fs.readdirSync(out).length)throw Error('Use a new output directory');fs.mkdirSync(out,{recursive:true});
const sleep=ms=>new Promise(r=>setTimeout(r,ms)),sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
let chrome,ws,driver,recording=false,recordingTask,server;const traces=[];
const tmp=fs.mkdtempSync('/tmp/rusty-workflow-');
function processDriver(language){const p=spawn(language==='rust'?path.join(__dirname,'target/release/desktop'):'node',language==='rust'?[]:[path.join(__dirname,'desktop-robotjs.cjs'),robot],{stdio:['pipe','pipe','pipe']});let pending=null,stderr='';p.stderr.on('data',d=>stderr+=d);createInterface({input:p.stdout}).on('line',line=>{if(pending){const {resolve,reject,timer}=pending;pending=null;clearTimeout(timer);try{const x=JSON.parse(line);x.ok?resolve(x.result):reject(Error(x.error))}catch(e){reject(e)}}});p.on('exit',code=>{if(pending){clearTimeout(pending.timer);pending.reject(Error('Driver exited '+code+' '+stderr));pending=null}});return {p,ask:v=>new Promise((resolve,reject)=>{if(pending)throw Error('Concurrent command');const timer=setTimeout(()=>{pending=null;reject(Error('Driver timeout'));p.kill()},15000);pending={resolve,reject,timer};p.stdin.write(JSON.stringify(v)+'\n')})}}
(async()=>{
 server=http.createServer((req,res)=>{res.setHeader('Content-Type','text/html');res.end(fs.readFileSync(path.join(__dirname,'workflow.html')))});await new Promise(r=>server.listen(0,'127.0.0.1',r));const url='http://127.0.0.1:'+server.address().port;
 chrome=spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',['--no-first-run','--no-default-browser-check','--disable-extensions','--disable-background-networking','--user-data-dir='+path.join(tmp,'profile'),'--remote-debugging-port=9440','--window-size=920,850','--window-position=160,80','--app='+url],{stdio:'ignore'});
 let tabs;for(let i=0;i<100;i++){try{tabs=await(await fetch('http://127.0.0.1:9440/json')).json();if(tabs.some(t=>t.type==='page'&&t.url.startsWith(url)))break}catch{}await sleep(100)}
 const tab=tabs.find(t=>t.type==='page'&&t.url.startsWith(url));assert(tab,'Missing owned app tab');ws=new WebSocket(tab.webSocketDebuggerUrl);await new Promise(r=>ws.addEventListener('open',r,{once:true}));let id=0;const pending=new Map();ws.addEventListener('message',e=>{const m=JSON.parse(e.data);if(pending.has(m.id)){const p=pending.get(m.id);pending.delete(m.id);m.error?p.reject(m.error):p.resolve(m.result)}});
 const send=(method,params={})=>new Promise((resolve,reject)=>{const n=++id;pending.set(n,{resolve,reject});ws.send(JSON.stringify({id:n,method,params}))});
 const ev=async expression=>{const r=await send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});if(r.exceptionDetails)throw Error(JSON.stringify(r.exceptionDetails));return r.result.value};
 await send('Page.enable');await send('Page.bringToFront');
 const windowInfo=await send('Browser.getWindowForTarget');await send('Browser.setWindowBounds',{windowId:windowInfo.windowId,bounds:{left:160,top:80,width:920,height:850}});await sleep(600);
 const checked=async()=>{const state=await ev('window.state()');assert(state.active,'Owned app lost focus; stopping native input');assert.equal(state.errors.length,0,'App rejected input: '+JSON.stringify(state.errors));return state};
 const observed=async test=>{for(let i=0;i<150;i++){await checked();if(await ev(test))return;await sleep(10)}throw Error('Observed app state timeout: '+test)};
 const geometry=async selector=>await ev(`(()=>{const r=document.querySelector(${JSON.stringify(selector)}).getBoundingClientRect();return {x:Math.round(screenX+r.x),y:Math.round(screenY+outerHeight-innerHeight+r.y),width:Math.floor(r.width),height:Math.floor(r.height)}})()`);
 const pointer=async(selector)=>{const g=await geometry(selector);return {op:'click',x:g.x+Math.floor(g.width/2),y:g.y+Math.floor(g.height/2)}};
 const profiles=[{id:'zero',label:'Keyboard delay: 0 ms',keyboard_ms:0,mouse_ms:10},{id:'paced',label:'Matched rate: 100 characters / second',keyboard_ms:0,mouse_ms:10,cpm:6000}];
 const manifest={date:new Date().toISOString(),cpu:execFileSync('sysctl',['-n','machdep.cpu.brand_string'],{encoding:'utf8'}).trim(),os:execFileSync('sw_vers',['-productVersion'],{encoding:'utf8'}).trim(),node:process.version,rustc:execFileSync('rustc',['--version'],{encoding:'utf8'}).trim(),source_commit:execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),robotjs_commit:execFileSync('git',['rev-parse','HEAD'],{cwd:robot,encoding:'utf8'}).trim(),sha256:Object.fromEntries(['desktop-robotjs.cjs','workflow.html','workflow-compare.cjs','src/bin/desktop.rs','target/release/desktop'].map(p=>[p,sha(path.join(__dirname,p))])),method:'Same eight-record form, exact field equality, same calls and settings (mouse delay 10ms, keyboard delay 0ms). One warmup per backend/profile, six measured runs each, alternating order. No video capture during measurements. Separate illustrative recordings afterward at 1x speed. Timer includes native delivery, app processing, verification IPC; startup/reset excluded. Zero-delay profile retains each library’s intrinsic behavior (Rust 10ms text-batch safety floor). Paced profile requests 6000 CPM from both APIs.',profiles,runs:[],videos:[]};
 const save=()=>fs.writeFileSync(path.join(out,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');save();
 async function run(profile,language,round,warmup,record=false){
  await send('Page.bringToFront');await ev(`window.reset('form',${JSON.stringify((language==='rust'?'Rusty Robot · Rust':'RobotJS · Node + C/C++')+' · '+profile.label)})`);await sleep(250);await checked();
  driver=processDriver(language);const permissions=await driver.ask({op:'probe'});assert.equal(permissions.accessibility,true);assert(permissions.displays>0);await driver.ask({op:'configure',keyboard_ms:profile.keyboard_ms,mouse_ms:profile.mouse_ms});
  const actions=[],native=async command=>{await checked();const start=performance.now();const result=await driver.ask(command);actions.push({command,result,ms:performance.now()-start});await checked();return result};
  const frames=[],dir=path.join(tmp,profile.id+'-'+language);let recordStart,startedAt,taskError=null;
  if(record){fs.mkdirSync(dir);recordStart=performance.now();recording=true;recordingTask=(async()=>{while(recording){const at=(performance.now()-recordStart)/1000;const shot=await send('Page.captureScreenshot',{format:'jpeg',quality:90});const file=path.join(dir,String(frames.length).padStart(5,'0')+'.jpg');fs.writeFileSync(file,Buffer.from(shot.data,'base64'));frames.push({at,file});await sleep(40)}})();await sleep(1100);startedAt=(performance.now()-recordStart)/1000;}
  await ev('window.start()');const hostStart=performance.now();
  try{
   const records=await ev('window.records');await native(await pointer('#name'));
   for(let i=0;i<records.length;i++){
    for(const [field,value] of ['name','email','note'].map((field,k)=>[field,records[i][k]])){
     await observed(`document.activeElement.id===${JSON.stringify(field)}`);await native({op:'type',text:value,...(profile.cpm?{cpm:profile.cpm}:{})});await observed(`document.getElementById(${JSON.stringify(field)}).value===${JSON.stringify(value)}`);await native({op:'key',key:'tab'});
    }
    await observed('document.activeElement.id==="save"');await native({op:'key',key:'enter'});await observed(`window.workflow.saved.length===${i+1}`);
   }
   const state=await checked();assert(state.finished);assert.deepEqual(state.saved,await ev('window.records'));assert(state.trusted>0);
  }catch(e){taskError=e.message;const state=await ev('window.state()');if(!state.active)throw e;await ev(`window.fail(${JSON.stringify(taskError)},'INPUT CHECK FAILED')`);}
  const state=await ev('window.state()');const actualFields=await ev("Object.fromEntries(['name','email','note'].map(k=>[k,document.getElementById(k).value]))");
  const result={profile:profile.id,language,round,warmup,recorded:record,verified:!taskError,seconds:taskError?null:state.seconds,failed_after_seconds:taskError?(performance.now()-hostStart)/1000:null,error:taskError,state,actualFields,permissions,actions};
  driver.p.stdin.end();driver=null;
  if(record){await sleep(1800);recording=false;await recordingTask;const concat=frames.map((f,i)=>`file '${f.file}'\nduration ${i+1<frames.length?frames[i+1].at-f.at:0.05}\n`).join('')+`file '${frames.at(-1).file}'\n`;fs.writeFileSync(path.join(dir,'frames.txt'),concat);const movie=path.join(dir,'run.mp4');execFileSync('ffmpeg',['-hide_banner','-loglevel','error','-y','-f','concat','-safe','0','-i',path.join(dir,'frames.txt'),'-vf',`trim=start=${Math.max(0,startedAt-1)},setpts=PTS-STARTPTS,fps=25,pad=ceil(iw/2)*2:ceil(ih/2)*2`,'-c:v','libx264','-crf','20','-pix_fmt','yuv420p','-an',movie]);result.movie=movie;}
  const file=profile.id+'-'+language+'-'+(record?'video':warmup?'warmup':'round-'+round)+'.json';fs.writeFileSync(path.join(out,file),JSON.stringify({...result,movie:undefined},null,2)+'\n');
  console.log(profile.id,language,record?'VIDEO':warmup?'WARMUP':'ROUND '+round,taskError?'FAIL '+JSON.stringify(actualFields):'PASS '+state.seconds.toFixed(3)+'s');
  return {...result,file};
 }
 for(const profile of profiles){
  for(const language of ['robotjs','rust']){const result=await run(profile,language,0,true);manifest.runs.push({file:result.file,profile:profile.id,language,round:0,warmup:true,verified:result.verified,seconds:result.seconds});save()}
  for(let round=1;round<=6;round++)for(const language of round%2?['robotjs','rust']:['rust','robotjs']){const result=await run(profile,language,round,false);manifest.runs.push({file:result.file,profile:profile.id,language,round,warmup:false,verified:result.verified,seconds:result.seconds});save()}
 }
 for(const profile of profiles){
  const runs=[];for(const language of ['robotjs','rust'])runs.push(await run(profile,language,0,false,true));
  const duration=Math.max(...runs.map(r=>r.seconds??r.failed_after_seconds))+3;
  execFileSync('ffmpeg',['-hide_banner','-loglevel','error','-y','-i',runs[0].movie,'-i',runs[1].movie,'-filter_complex',`[0:v]tpad=stop_mode=clone:stop_duration=${duration},trim=duration=${duration},setpts=PTS-STARTPTS[a];[1:v]tpad=stop_mode=clone:stop_duration=${duration},trim=duration=${duration},setpts=PTS-STARTPTS[b];[a][b]hstack=inputs=2[v]`,'-map','[v]','-c:v','libx264','-crf','20','-pix_fmt','yuv420p','-movflags','+faststart','-an',path.join(out,profile.id+'.mp4')]);execFileSync('ffmpeg',['-hide_banner','-loglevel','error','-y','-sseof','-0.5','-i',path.join(out,profile.id+'.mp4'),'-frames:v','1',path.join(out,profile.id+'.png')]);manifest.videos.push({profile:profile.id,results:runs.map(r=>({file:r.file,language:r.language,verified:r.verified,seconds:r.seconds})),sha256:sha(path.join(out,profile.id+'.mp4'))});save();
 }
 await send('Browser.close');ws.close();server.close();console.log('Finished controlled comparisons and separate illustrative recordings');
})().catch(async e=>{console.error(e);recording=false;if(recordingTask)await recordingTask.catch(()=>{});if(driver)driver.p.kill();if(chrome)chrome.kill();if(ws)ws.close();if(server)server.close();process.exitCode=1});
