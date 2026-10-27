#!/usr/bin/env node
const {spawnSync}=require('node:child_process');
const {mkdirSync,readFileSync,writeFileSync}=require('node:fs');
const {resolve,join}=require('node:path');
const os=require('node:os');
const root=resolve(__dirname,'..'),out=join(root,'benchmarks/2026-10-27-consumers/processing');mkdirSync(out,{recursive:true});
if(!process.env.BENCHMARK_JIMP)throw Error('Set BENCHMARK_JIMP to the absolute consumer node_modules/jimp directory');
const runs=[];
for(let round=1;round<=3;round++)for(const backend of round%2?['robotjs','rusty-robot']:['rusty-robot','robotjs']){
  const file=join(out,`${round}-${backend}.json`),result=spawnSync(process.execPath,[join(root,'benchmarks/consumer-harness/processing.cjs'),file],{cwd:root,env:{...process.env,BENCHMARK_BACKEND:backend,BENCHMARK_ADDON:join(root,'target/rusty-robot.node')},stdio:'inherit',timeout:120000});
  if(result.error||result.status!==0)throw Error(String(result.error||result.status));
  runs.push({round,...JSON.parse(readFileSync(file))});console.log(`Round ${round}: ${backend} processing passed`);
}
writeFileSync(join(out,'raw.json'),JSON.stringify({measured_at:new Date().toISOString(),cpu:os.cpus()[0].model,platform:os.platform(),release:os.release(),scope:'PNG decode/resize/JPEG encode only; no desktop capture or socket transport. robotjs label means the original Jimp image-processing dependency, not RobotJS itself.',runs},null,2));
