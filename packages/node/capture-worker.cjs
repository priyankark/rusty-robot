'use strict';
// Private media worker. Input ownership stays in the host.
const fs = require('node:fs');
const path = require('node:path');
const { Worker } = require('node:worker_threads');
const directory = process.argv[3];
// This file is an implementation detail, not a CLI that accepts a cleanup path.
// Only remove the private directory containing this worker's copied executable.
if (!directory || !path.basename(directory).startsWith('rusty-robot-capture-') ||
    fs.realpathSync(directory) !== fs.realpathSync(path.dirname(process.execPath))) {
  throw new Error('Capture worker must be launched from its private package runtime');
}
// A separate JS thread still runs if loading the addon or a native call blocks
// this event loop. It also reaps the worker if its host is killed abruptly.
const watchdog = new Worker(path.join(__dirname, 'capture-watchdog.cjs'), {
  workerData: { directory, parentPid: process.ppid, timeoutMs: Number(process.argv[4]) },
  execArgv: [],
});
watchdog.unref();
watchdog.on('error', () => process.exit(1));
function cleanup() {
  try { fs.rmSync(directory, { recursive: true, force: true }); } catch { /* Parent also cleans up. */ }
}
process.on('exit', cleanup);
process.on('disconnect', () => process.exit(0));
function send(message) {
  if (process.connected) process.send(message, error => { if (error) process.exit(1); });
}
try {
  const native = require(process.argv[2]); // Rechecks the packaged binary's SHA-256.
  let busy = false;
  process.on('message', async message => {
    if (!['capture', 'resize'].includes(message?.type) || !Number.isSafeInteger(message.id) || !Array.isArray(message.args)) return;
    if (busy) { send({ id: message.id, error: { message: 'Capture worker is busy' } }); return; }
    busy = true;
    watchdog.postMessage('active');
    try {
      const operation = message.type === 'capture' ? native.captureFrame : native.resizeImage;
      send({ id: message.id, frame: await operation(...message.args) });
    }
    catch (error) { send({ id: message.id, error: { message: error.message, code: error.code } }); }
    finally { busy = false; watchdog.postMessage('idle'); }
  });
  send({ type: 'ready' });
  watchdog.postMessage('idle');
} catch (error) {
  send({ type: 'startup-error', error: { message: error.message, code: error.code } });
}
