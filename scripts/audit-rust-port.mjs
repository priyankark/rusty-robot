// Registration-only audit: never invokes an upstream test body or native addon.
import fs from 'node:fs';
import vm from 'node:vm';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const repoRoot=path.join(path.dirname(fileURLToPath(import.meta.url)), '..');
const upstream=process.argv[2] ? path.resolve(process.argv[2]) : path.resolve(repoRoot,'../robotjs');
process.chdir(repoRoot);
const manifest=JSON.parse(fs.readFileSync('tests/upstream-coverage.json','utf8'));
const mock=new Proxy(function(){},{get(){return mock;}});
const registered=[];
const sources=fs.readdirSync(path.join(upstream,'test'),{recursive:true}).map(f=>f.replaceAll(path.sep,'/')).filter(f=>f.endsWith('.js') && !f.startsWith('helpers/')).map(f=>'test/'+f).sort();
assert.deepEqual(sources,[...new Set(manifest.cases.map(c=>c.source))].sort(),'unmapped upstream test file');
for(const source of sources) {
  const record=name=>registered.push({source,name});
  vm.runInNewContext(fs.readFileSync(path.join(upstream,source),'utf8'),{
    require:()=>mock,describe:(_,run)=>run(),it:record,xit:record,
    beforeAll(){},afterAll(){},beforeEach(){},afterEach(){},process:{platform:'darwin'}
  },{filename:source,timeout:1000});
}
const key=c=>`${c.source}: ${c.name}`;
assert.deepEqual(registered.map(key).sort(),manifest.cases.map(key).sort(),'upstream cases changed');
for(const [source,expected] of Object.entries(manifest.source_sha256)) {
  assert.equal(createHash('sha256').update(fs.readFileSync(path.join(upstream,source))).digest('hex'),expected,`${source} changed; review assertions and update mapping`);
}
for(const c of manifest.cases) {
  assert(c.rust.length>0,`missing port: ${key(c)}`);
  for(const port of c.rust) {
    const content=fs.readFileSync(port.file,'utf8');
    if(port.test) assert(new RegExp(`(?:fn\\s+${port.test}\\s*\\(|invalid_metadata!\\(\\s*${port.test}\\s*,)`).test(content),`missing Rust test ${port.test}`);
    else assert(content.includes(`<!-- contract: ${port.contract} -->`),`missing type contract ${port.contract}`);
  }
}
console.log(`Mapped all ${registered.length} upstream cases at ${manifest.revision}; source hashes unchanged.`);
