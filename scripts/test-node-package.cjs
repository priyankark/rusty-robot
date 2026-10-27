#!/usr/bin/env node
'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { execFileSync, spawnSync } = require('node:child_process');
const { deflateSync } = require('node:zlib');
function crc32(bytes) {
  let value = 0xffffffff;
  for (const byte of bytes) { value ^= byte; for (let i=0;i<8;i++) value = (value >>> 1) ^ ((value & 1) ? 0xedb88320 : 0); }
  return (value ^ 0xffffffff) >>> 0;
}
function png() {
  function chunk(name, bytes) { const type=Buffer.from(name), out=Buffer.alloc(bytes.length+12);out.writeUInt32BE(bytes.length);type.copy(out,4);bytes.copy(out,8);out.writeUInt32BE(crc32(Buffer.concat([type,bytes])),out.length-4);return out; }
  const header=Buffer.alloc(13);header.writeUInt32BE(64);header.writeUInt32BE(64,4);header[8]=8;header[9]=2;
  const pixels=Buffer.alloc(64*(64*3+1));for(let y=0;y<64;y++)for(let x=0;x<64;x++)pixels[y*(64*3+1)+1+x*3]=255;
  return Buffer.concat([Buffer.from('89504e470d0a1a0a','hex'),chunk('IHDR',header),chunk('IDAT',deflateSync(pixels)),chunk('IEND',Buffer.alloc(0))]);
}
(async () => {
  if (process.argv[2] === '--smoke') {
    const packageRoot = path.resolve(process.argv[3]);
    const native=require(packageRoot);
    assert.equal(typeof native.getPermissions,'function');assert.equal(typeof native.typeString,'function');
    assert.equal(typeof native.getPermissions(),'object');
    const encoded=await native.resizeImage(png(),32,null,85,'jpeg');
    assert.deepEqual([encoded.width,encoded.height],[32,32]);assert.equal(encoded.data.subarray(0,2).toString('hex'),'ffd8');
    const output=await native.resizeImage(png(),16,null,85,'png');
    assert.equal(output.data.subarray(0,8).toString('hex'),'89504e470d0a1a0a');assert.equal(output.data.readUInt32BE(16),16);
    await assert.rejects(native.resizeImage(Buffer.from('invalid image'),16),/JPEG|PNG|image|format/i);
    return;
  }
  let archive = path.resolve(process.argv[2]);
  if (fs.statSync(archive).isDirectory()) {
    const files = fs.readdirSync(archive).filter(name => name.endsWith('.tgz'));
    assert.equal(files.length, 1, 'expected one native package archive');
    archive = path.join(archive, files[0]);
  }
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'rusty-robot-installed-'));
  try {
    const npm = process.platform === 'win32' ? path.join(path.dirname(process.execPath), 'node_modules/npm/bin/npm-cli.js') : path.resolve(path.dirname(process.execPath), '../lib/node_modules/npm/bin/npm-cli.js');
    const args = ['install', '--prefix', directory, '--ignore-scripts', '--no-audit', '--no-fund', archive];
    if (fs.existsSync(npm)) execFileSync(process.execPath, [npm, ...args], {stdio:'inherit'});
    else { // Portable entry point for npm installations managed by fnm/nvm.
      const result = spawnSync(process.platform === 'win32' ? 'npm.cmd' : 'npm', args, {stdio:'inherit', shell:process.platform === 'win32'});
      if (result.status !== 0) throw new Error('Clean package installation failed');
    }
    const packageRoot=path.join(directory,'node_modules/@priyankark/rusty-robot');
    execFileSync(process.execPath, [__filename, '--smoke', packageRoot], { stdio: 'inherit' });
    const target=require(path.join(packageRoot,'platform.cjs')).selectTarget();
    // A fresh host process must reject corruption before dlopen executes it.
    const binary=path.join(packageRoot,'prebuilds',target+'.node');
    const fd=fs.openSync(binary,'r+');fs.writeSync(fd,Buffer.from([0]),0,1,0);fs.closeSync(fd);
    const result=spawnSync(process.execPath,['-e','require(process.argv[1])',packageRoot],{encoding:'utf8'});
    assert.notEqual(result.status,0);assert.match(result.stderr,/integrity check failed/);
    console.log(`Clean installed package: ${target}, Node ${process.versions.node}; media and corruption checks passed`);
  } finally { fs.rmSync(directory,{recursive:true,force:true}); }
})().catch(error=>{console.error(error);process.exitCode=1;});
