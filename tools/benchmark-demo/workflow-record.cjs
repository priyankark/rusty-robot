// Records actual OS input into an owned Chrome app. Read-only DOM assertions verify delivery.
const fs=require('node:fs'),path=require('node:path'),http=require('node:http'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const {spawn,execFileSync}=require('node:child_process');const {createInterface}=require('node:readline');
const root=path.resolve(__dirname,'../..'),[robotArg,outArg]=process.argv.slice(2);
if(!robotArg||!outArg)throw Error('Usage: node workflow-record.cjs ROBOTJS NEW_OUTPUT_DIRECTORY');
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
 const manifest={date:new Date().toISOString(),cpu:execFileSync('sysctl',['-n','machdep.cpu.brand_string'],{encoding:'utf8'}).trim(),os:execFileSync('sw_vers',['-productVersion'],{encoding:'utf8'}).trim(),node:process.version,rustc:execFileSync('rustc',['--version'],{encoding:'utf8'}).trim(),source_commit:execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),robotjs_commit:execFileSync('git',['rev-parse','HEAD'],{cwd:robot,encoding:'utf8'}).trim(),sha256:{rust:sha(path.join(__dirname,'target/release/desktop')),robotjs:sha(path.join(robot,'build/Release/robotjs.node')),fixture:sha(path.join(__dirname,'workflow.html')),recorder:sha(__filename)},method:'Sequential real desktop recordings, aligned at task start and combined side by side at 1x speed. Native input only; DOM reads verify delivery. Both retain default 10ms delays; text batching differs. Checks, IPC, rendering and recording are included. No isolated benchmark claim.',runs:[]};
 for(const mode of ['form','targets']){
  const runs=[];
  for(const language of ['robotjs','rust']){
   await send('Page.bringToFront');await ev(`window.reset(${JSON.stringify(mode)},${JSON.stringify(language==='rust'?'Rusty Robot · Rust implementation':'RobotJS · Node.js + C/C++ addon')})`);await sleep(350);await checked();
   driver=processDriver(language);const permissions=await driver.ask({op:'probe'});assert.equal(permissions.accessibility,true);assert.equal(permissions.capture,true);assert(permissions.displays>0);console.log(mode,language,'permissions and focus verified');
   const actions=[],native=async command=>{await checked();const begun=performance.now();let result;try{result=await driver.ask(command)}catch(e){actions.push({command,error:e.message,ms:performance.now()-begun});throw e}actions.push({command,result,ms:performance.now()-begun});await checked();return result};
   const dir=path.join(tmp,mode+'-'+language);fs.mkdirSync(dir);const frames=[],start=performance.now();recording=true;
   recordingTask=(async()=>{while(recording){const at=(performance.now()-start)/1000;const shot=await send('Page.captureScreenshot',{format:'jpeg',quality:90});const file=path.join(dir,String(frames.length).padStart(5,'0')+'.jpg');fs.writeFileSync(file,Buffer.from(shot.data,'base64'));frames.push({at,file});await sleep(40)}})();
   await sleep(1100);const startedAt=(performance.now()-start)/1000;await ev('window.start()');
   if(mode==='form'){
    const records=await ev('window.records');await native(await pointer('#name'));
    for(let i=0;i<records.length;i++){
     for(const [field,value] of ['name','email','note'].map((field,k)=>[field,records[i][k]])){
      await native({op:'type',text:value});await observed(`document.getElementById(${JSON.stringify(field)}).value===${JSON.stringify(value)}`);await native({op:'key',key:'tab'});
     }
     await native({op:'key',key:'enter'});await observed(`window.workflow.saved.length===${i+1}`);
    }
   }else{
    for(let i=0;i<48;i++){
     await ev('new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');
     const g=await geometry('#board');if(i===0)console.log('Board geometry',g);let found;try{found=await native({op:'find',...g})}catch(e){if(language==='robotjs'&&e.message==='Failed to capture the requested screen rect.'){await ev('window.fail('+JSON.stringify(e.message)+')');break}throw e}assert(found.x>=g.x&&found.x<g.x+g.width&&found.y>=g.y&&found.y<g.y+g.height,'Captured point outside owned board');await native({op:'click',x:found.x,y:found.y});await observed(`window.workflow.hits===${i+1}`);
    }
   }
   const state=await checked();assert(state.finished);assert.equal(state.misses,0);if(!state.failure)assert(state.trusted>0);if(mode==='form')assert.deepEqual(state.saved,await ev('window.records'));else if(!state.failure)assert.equal(state.hits,48);else assert.equal(state.hits,0);
   const poster=await send('Page.captureScreenshot',{format:'png'});fs.writeFileSync(path.join(out,mode+'-'+language+'.png'),Buffer.from(poster.data,'base64'));
   await sleep(1800);recording=false;await recordingTask;driver.p.stdin.end();driver=null;
   const concat=frames.map((f,i)=>`file '${f.file}'\nduration ${i+1<frames.length?frames[i+1].at-f.at:0.05}\n`).join('')+`file '${frames.at(-1).file}'\n`;fs.writeFileSync(path.join(dir,'frames.txt'),concat);
   const movie=path.join(dir,'run.mp4');execFileSync('ffmpeg',['-hide_banner','-loglevel','error','-y','-f','concat','-safe','0','-i',path.join(dir,'frames.txt'),'-vf',`trim=start=${Math.max(0,startedAt-1)},setpts=PTS-STARTPTS,fps=25,pad=ceil(iw/2)*2:ceil(ih/2)*2`,'-c:v','libx264','-crf','20','-pix_fmt','yuv420p','-an',movie]);
   const run={mode,language,seconds:state.seconds,verified:!state.failure,state,permissions,actions,frames:frames.length,start_offset:startedAt};fs.writeFileSync(path.join(out,mode+'-'+language+'.json'),JSON.stringify(run,null,2)+'\n');runs.push({...run,movie});console.log(mode,language,state.failure?'CAPTURE FAILED':'PASS',state.seconds.toFixed(3)+'s');
  }
  const duration=Math.max(...runs.map(r=>r.seconds))+3;
  execFileSync('ffmpeg',['-hide_banner','-loglevel','error','-y','-i',runs[0].movie,'-i',runs[1].movie,'-filter_complex',`[0:v]tpad=stop_mode=clone:stop_duration=${duration},trim=duration=${duration},setpts=PTS-STARTPTS[a];[1:v]tpad=stop_mode=clone:stop_duration=${duration},trim=duration=${duration},setpts=PTS-STARTPTS[b];[a][b]hstack=inputs=2[v]`,'-map','[v]','-c:v','libx264','-crf','20','-pix_fmt','yuv420p','-movflags','+faststart','-an',path.join(out,mode+'.mp4')]);
  execFileSync('ffmpeg',['-hide_banner','-loglevel','error','-y','-sseof','-0.5','-i',path.join(out,mode+'.mp4'),'-frames:v','1',path.join(out,mode+'.png')]);
  manifest.runs.push({mode,results:runs.map(({movie,actions,...run})=>run),video_sha256:sha(path.join(out,mode+'.mp4'))});fs.writeFileSync(path.join(out,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');
 }
 fs.writeFileSync(path.join(out,'manifest.json'),JSON.stringify(manifest,null,2)+'\n');await send('Browser.close');ws.close();server.close();console.log('Recorded and verified both desktop workflows');
})().catch(async e=>{console.error(e);recording=false;if(recordingTask)await recordingTask.catch(()=>{});if(driver)driver.p.kill();if(chrome)chrome.kill();if(ws)ws.close();if(server)server.close();process.exitCode=1});
