#!/usr/bin/env node
'use strict';
const { readFileSync, writeFileSync, copyFileSync } = require('node:fs');
const { resolve, join } = require('node:path');
const { createHash } = require('node:crypto');
const { execFileSync } = require('node:child_process');
const root = resolve(__dirname, '..'), directory = join(root, 'packages/node');
const { targets } = require('../packages/node/platform.cjs');
const metadata = require('../packages/node/package.json');
const binaries = {};
for (const target of targets) {
  const bytes = readFileSync(join(directory, 'prebuilds', `${target}.node`));
  if (bytes.length < 10000) throw new Error(`Invalid native binary: ${target}`);
  binaries[target] = { sha256: createHash('sha256').update(bytes).digest('hex'), size: bytes.length };
}
if (process.argv.includes('--verify')) {
  const manifest = require('../packages/node/manifest.json');
  if (manifest.version !== metadata.version || JSON.stringify(manifest.binaries) !== JSON.stringify(binaries)) throw new Error('Native package manifest does not match its binaries/version');
  console.log(`Verified all ${targets.length} packaged native binaries`);
} else {
  const revision = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim();
  writeFileSync(join(directory, 'manifest.json'), JSON.stringify({ version: metadata.version, revision, napi: 6, binaries }, null, 2) + '\n');
  for (const name of ['LICENSE.md', 'NOTICE.md']) copyFileSync(join(root, name), join(directory, name));
  copyFileSync(join(root, 'crates/rusty-robot-node/index.d.ts'), join(directory, 'index.d.ts'));
  console.log(`Assembled ${metadata.name}@${metadata.version} from ${revision}`);
}
