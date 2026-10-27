'use strict';
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { createHash } = require('node:crypto');
const { selectTarget } = require('./platform.cjs');
const target = selectTarget();
const manifest = require('./manifest.json');
const file = join(__dirname, 'prebuilds', `${target}.node`);
const expected = manifest.binaries[target];
if (!expected || createHash('sha256').update(readFileSync(file)).digest('hex') !== expected.sha256) {
  throw new Error(`Rusty Robot binary integrity check failed for ${target}. Reinstall the pinned package.`);
}
try { module.exports = require(file); }
catch (cause) { throw new Error(`Rusty Robot could not load its ${target} binary: ${cause.message}`, { cause }); }
