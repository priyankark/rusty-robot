'use strict';
const fs = require('node:fs');
const path = require('node:path');
exports.captureFrame = async (width, quality, previous, force, format) => {
  fs.appendFileSync(path.join(__dirname, 'calls.jsonl'), JSON.stringify({width, pid:process.pid, executable:process.execPath})+'\n');
  if (width === 8) Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0); // Synchronous native hang.
  if (width === 7) process.exit(17);
  if (width === 9) throw Object.assign(new Error('permission refused'), {code:'GenericFailure'});
  await new Promise(resolve => setTimeout(resolve, 10));
  return {width, quality, previous, force, format, pid:process.pid, executable:process.execPath,
    data: previous === 'same' && !force ? undefined : Buffer.from([1,2,3]), fingerprint:'same'};
};
exports.resizeImage = async (input, width, height, quality, format) => {
  await new Promise(resolve=>setTimeout(resolve,100));
  return {width,height,quality,format,data:input};
};
