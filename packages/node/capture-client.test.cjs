'use strict';
const { test, describe } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawn } = require('node:child_process');
const { once } = require('node:events');
const { needsIsolation, createCaptureClient, isolateCapture } = require('./capture-client.cjs');
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
async function until(predicate) {
  const deadline=Date.now()+5000;
  while (!predicate()) { if(Date.now()>deadline) throw Error('condition timed out'); await delay(20); }
}
function fixture(t) {
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'robot-transport-test-'));
  const binding=path.join(root,'binding.cjs');
  fs.copyFileSync(path.join(__dirname,'fixtures/capture.cjs'),binding);
  t.after(()=>fs.rmSync(root,{recursive:true,force:true}));
  const clients=[];
  t.after(()=>clients.forEach(client=>client.close()));
  return {root,binding, client(options={}) { const client=createCaptureClient(binding,{temporaryRoot:root,...options});clients.push(client);return client; },
    calls() {try{return fs.readFileSync(path.join(root,'calls.jsonl'),'utf8').trim().split('\n').map(JSON.parse);}catch{return[];}},
    directories() {return fs.readdirSync(root).filter(name=>name.startsWith('rusty-robot-capture-'));} };
}
test('only affected macOS hosts use isolation',()=>{
  assert(needsIsolation('darwin','27.0.0'));
  assert(!needsIsolation('darwin','26.1.0'));
  assert(!needsIsolation('linux','27.0'));
  assert(!needsIsolation('win32','27.0'));
});
// The copied-runtime transport is not enabled on Windows, where running files
// cannot be unlinked. Platform selection above is still checked there.
describe('isolated worker lifecycle on Unix', {skip:process.platform==='win32'},()=>{
test('distinct clients use private runtime copies, reuse workers and preserve frame bytes/arguments', async t=>{
  const f=fixture(t), a=f.client(), b=f.client();
  const [first,second]=await Promise.all([a.capture([123,85,null,true,'png']),b.capture([234,81,'old',false,'jpeg'])]);
  assert.notEqual(first.executable,second.executable);
  assert.notEqual(first.executable,process.execPath);
  assert(Buffer.isBuffer(first.data));assert.deepEqual(first.data,Buffer.from([1,2,3]));
  assert.deepEqual([second.width,second.quality,second.previous,second.force,second.format],[234,81,'old',false,'jpeg']);
  if(process.platform!=='win32'){
    assert.equal(fs.statSync(first.executable).mode&0o777,0o700);
    assert.equal(fs.statSync(path.dirname(first.executable)).mode&0o777,0o700);
  }
  const unchanged=await a.capture([123,85,'same',false,'png']);
  assert.equal(unchanged.pid,first.pid);assert.equal(unchanged.data,undefined);
  a.close();b.close();await until(()=>f.directories().length===0);
});
test('native capture failures preserve their error and discard the worker before a later explicit capture',async t=>{
  const f=fixture(t), c=f.client();
  await assert.rejects(c.capture([9]),{message:'permission refused',code:'GenericFailure'});
  await until(()=>f.directories().length===0);
  assert.equal((await c.capture([42])).width,42);
  assert.notEqual(f.calls()[0].pid,f.calls()[1].pid);
  c.close();await until(()=>f.directories().length===0);
});
test('synchronous hang times out, rejects queued work without replay, cleans up and recovers',async t=>{
  const f=fixture(t), c=f.client({timeoutMs:5000});
  const first=c.capture([8]), queued=c.capture([31]);
  const outcomes=await Promise.allSettled([first,queued]);
  assert(outcomes.every(result=>result.status==='rejected'));
  assert.match(outcomes[0].reason.message,/exceeded|exited/);
  await until(()=>f.directories().length===0);
  assert.deepEqual(f.calls().map(call=>call.width),[8]);
  assert.equal((await c.capture([32])).width,32);
  c.close();await until(()=>f.directories().length===0);
});
test('worker crashes reject pending capture and clean up',async t=>{
  const f=fixture(t), c=f.client();
  await assert.rejects(c.capture([7]),/exited|disconnected/);
  await until(()=>f.directories().length===0);
});
test('startup failures reject without falling back to capture in the host',async t=>{
  const f=fixture(t), c=f.client({executable:path.join(f.root,'missing-node')});
  await assert.rejects(c.capture([1]),/could not start/);
  await until(()=>f.directories().length===0);
  assert.deepEqual(f.calls(),[]);
});
test('worker-side native load errors are surfaced and cleaned up',async t=>{
  const f=fixture(t), c=f.client();
  fs.writeFileSync(f.binding,"throw Error('binary integrity check failed');");
  await assert.rejects(c.capture([1]),/could not load Rusty Robot: binary integrity check failed/);
  await until(()=>f.directories().length===0);
});
test('a runtime that cannot execute fails without leaving its private directory', {skip:process.platform==='win32'}, async t=>{
  const f=fixture(t), executable=path.join(f.root,'unlaunchable');
  fs.writeFileSync(executable,'#!/nonexistent/rusty-robot-test-interpreter\n');
  const c=f.client({executable});
  await assert.rejects(c.capture([1]),/failed|exited/);
  await until(()=>f.directories().length===0);
});
test('capture and resize share two permits; validation and input remain in the host',async t=>{
  const f=fixture(t);
  const native={validateCaptureFrame(width){if(width===0)throw Error('invalid dimension');},
    validateResizeImage(){},resizeImage(){throw Error('resize must use the shared media worker');},keyTap(){return 'parent input';}};
  const robot=isolateCapture(native,f.binding,{temporaryRoot:f.root,idleMs:20});
  assert.equal(robot.keyTap,native.keyTap);
  assert.throws(()=>robot.captureFrame(0),/dimension/);
  assert.equal(f.directories().length,0);
  const buffer=Buffer.from([1]);
  const capture=robot.captureFrame(4),resize=robot.resizeImage(buffer,1);
  buffer.fill(99);
  assert.throws(()=>robot.captureFrame(5),/queue is full/);
  assert.throws(()=>robot.resizeImage(Buffer.from([1]),1),/queue is full/);
  await capture;assert.deepEqual((await resize).data,Buffer.from([1]));
  assert.equal((await robot.captureFrame(6)).width,6);
  await until(()=>f.directories().length===0);
});
test('idle workers do not keep their host process alive',async t=>{
  const f=fixture(t);
  const program=`const{createCaptureClient}=require(${JSON.stringify(require.resolve('./capture-client.cjs'))});
    createCaptureClient(${JSON.stringify(f.binding)},{temporaryRoot:${JSON.stringify(f.root)}}).capture([3]).then(()=>console.log('done'));`;
  const child=spawn(process.execPath,['-e',program],{stdio:['ignore','pipe','pipe']});
  t.after(()=>child.kill('SIGKILL'));
  const watchdog=setTimeout(()=>child.kill('SIGKILL'),5000);
  const [code,signal]=await once(child,'exit');clearTimeout(watchdog);
  assert.equal(code,0);assert.equal(signal,null);
  await until(()=>f.directories().length===0);
});
test('a blocked worker is reaped when its host is killed', {skip:process.platform==='win32'}, async t=>{
  const f=fixture(t);
  const program=`const{createCaptureClient}=require(${JSON.stringify(require.resolve('./capture-client.cjs'))});
    createCaptureClient(${JSON.stringify(f.binding)},{temporaryRoot:${JSON.stringify(f.root)}}).capture([8]).catch(()=>{});`;
  const host=spawn(process.execPath,['-e',program],{stdio:'ignore'});
  t.after(()=>host.kill('SIGKILL'));
  await until(()=>f.calls().length===1);
  const exited=once(host,'exit');host.kill('SIGKILL');await exited;
  await until(()=>f.directories().length===0);
  const pid=f.calls()[0].pid;
  // A reparented zombie can remain until init reaps it; its runtime must already be gone.
  const {spawnSync}=require('node:child_process');
  await until(()=>{const r=spawnSync('ps',['-p',String(pid),'-o','stat='],{encoding:'utf8'});return r.status!==0||r.stdout.trim().startsWith('Z');});
});

});
