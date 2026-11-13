'use strict';
const { parentPort, workerData } = require('node:worker_threads');
const fs = require('node:fs');
function terminate() {
  try { fs.rmSync(workerData.directory, { recursive: true, force: true }); } catch { /* Parent also cleans up. */ }
  process.kill(process.pid, 'SIGKILL');
}
let deadline = setTimeout(terminate, workerData.timeoutMs);
parentPort.on('message', state => {
  clearTimeout(deadline);
  if (state === 'active') deadline = setTimeout(terminate, workerData.timeoutMs);
});
setInterval(() => {
  if (process.ppid !== workerData.parentPid) terminate();
}, 250);
