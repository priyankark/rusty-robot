#!/usr/bin/env python3
"""Benchmark the Rust release build against RobotJS's public JS/native API."""
import argparse
import csv
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import statistics
import struct
import subprocess
import zlib

ROOT = Path(__file__).resolve().parent.parent
UPSTREAM = '0af6073e1f1bcff865ffe52801c71647829cdc1c'
CASES = ['color_at','find_color_early','find_color_late','find_color_absent',
         'count_color_absent','count_color_dense','find_colors_dense','find_image_late',
         'find_image_absent','find_image_tolerant','count_image_dense','find_images_dense',
         'load_bmp','load_png','save_bmp','save_png','get_mouse_pos','get_displays',
         'get_pixel_color','capture_small','capture_full']
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--robotjs', type=Path, default=ROOT/'target/benchmark/robotjs')
parser.add_argument('--rounds', type=int, default=3)
parser.add_argument('--cases', nargs='+', choices=CASES, default=CASES)
parser.add_argument('--skip-build', action='store_true')
args = parser.parse_args()
if args.rounds < 1: parser.error('--rounds must be positive')
output = ROOT/'target/benchmark'
fixtures = output/'fixtures'
fixtures.mkdir(parents=True, exist_ok=True)
robot = args.robotjs.resolve()
if not args.skip_build:
    if not robot.exists():
        subprocess.run(['git','clone','https://github.com/octalmage/robotjs.git',str(robot)],check=True)
        subprocess.run(['git','checkout','--detach',UPSTREAM],cwd=robot,check=True)
    revision = subprocess.check_output(['git','rev-parse','HEAD'],cwd=robot,text=True).strip()
    if revision != UPSTREAM:
        raise SystemExit(f'Expected RobotJS {UPSTREAM}, found {revision}; use a clean pinned checkout')
    env=os.environ.copy();env['ROBOTJS_ENABLE_PNG']='1'
    for name,cmd in [('robotjs-install',['npm','ci','--ignore-scripts']),('robotjs-build',['npm','run','build'])]:
        with (output/(name+'.log')).open('w') as log:
            subprocess.run(cmd,cwd=robot,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
    subprocess.run(['cargo','build','--release','--locked','--example','benchmark'],cwd=ROOT,check=True)

revision = subprocess.check_output(['git','rev-parse','HEAD'],cwd=robot,text=True).strip()
if revision != UPSTREAM:
    raise SystemExit(f'Expected RobotJS {UPSTREAM}, found {revision}')

# Canonical files shared by both processes. No fixture generation enters timings.
w,h=1920,1080
canvas=bytearray(w*h*4)
for y in range(h):
    row=bytearray()
    for x in range(w): row.extend(((x*17+y*11)%128,(x*7+y*13)%128,(x*3+y*5)%128,255))
    canvas[y*w*4:(y+1)*w*4]=row
needle=bytearray()
for y in range(32):
    for x in range(32): needle.extend((128+(x*13+y*3)%128,128+(x*5+y*7)%128,128+(x*11+y*17)%128,255))
needle[:4]=bytes([255,0,255,255])
for y in range(32):
    start=((1016+y)*w+1856)*4
    canvas[start:start+128]=needle[y*128:(y+1)*128]
absent=bytearray(needle)
absent[:4]=bytes([255,255,0,255]);absent[-4:]=bytes([255,255,0,255])
for name,data in [('canvas',canvas),('needle',needle),('absent',absent),
                  ('dense',bytes([85,68,51,255])*256*256),('tile',bytes([85,68,51,255])*16)]:
    (fixtures/(name+'.bgra')).write_bytes(data)
rgb=bytearray(w*h*3)
for i in range(w*h): rgb[i*3:i*3+3]=canvas[i*4:i*4+3][::-1]
header=bytearray(54)
header[:2]=b'BM'
for offset,value in [(2,54+w*h*3),(10,54),(14,40),(18,w),(22,h),(34,w*h*3)]: struct.pack_into('<I',header,offset,value)
struct.pack_into('<HH',header,26,1,24)
with (fixtures/'source.bmp').open('wb') as f:
    f.write(header)
    for y in reversed(range(h)):
        row=canvas[y*w*4:(y+1)*w*4]
        f.write(bytes(v for i,v in enumerate(row) if i%4!=3))
def chunk(kind,data): return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data))
png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w,h,8,2,0,0,0))
png+=chunk(b'IDAT',zlib.compress(b''.join(b'\0'+rgb[y*w*3:(y+1)*w*3] for y in range(h)),6))+chunk(b'IEND',b'')
(fixtures/'source.png').write_bytes(png)
def version(cmd): return subprocess.check_output(cmd,text=True,cwd=ROOT).strip()
metadata={'platform':platform.platform(),'processor':platform.processor(),
          'node':version(['node','--version']),'rustc':version(['rustc','--version']),
          'rust_profile':'release (opt-level=3, default Cargo settings)',
          'rustflags':os.environ.get('RUSTFLAGS',''),
          'robotjs_build':'node-gyp Release, -O3, ROBOTJS_ENABLE_PNG=1',
          'clang':version(['clang','--version']).splitlines()[0],
          'robotjs_commit':version(['git','-C',str(robot),'rev-parse','HEAD']),
          'rounds':args.rounds,'samples_per_round':7,'target_batch_ms':40,
          'fixtures_sha256':{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in fixtures.iterdir() if p.name in ['canvas.bgra','source.bmp','source.png','needle.bgra']}}
if platform.system()=='Darwin':
    metadata['cpu']=version(['sysctl','-n','machdep.cpu.brand_string'])
    metadata['memory_bytes']=version(['sysctl','-n','hw.memsize'])
probe="const r=require(process.argv[1]);const p=require('node:path'); console.log(JSON.stringify(Object.fromEntries(['bmp','png'].map(e=>{const b=r.image.load(p.join(process.argv[2],'source.'+e));return [e,{bitsPerPixel:b.bitsPerPixel,bytesPerPixel:b.bytesPerPixel,byteWidth:b.byteWidth}]}))))"
metadata['robotjs_decoded_formats']=json.loads(subprocess.check_output(['node','-e',probe,str(robot),str(fixtures)],text=True))
metadata['rust_decoded_format']={'bitsPerPixel':32,'bytesPerPixel':4,'byteWidth':1920*4}
fingerprint=hashlib.sha256()
for source in sorted([ROOT/'Cargo.toml',ROOT/'Cargo.lock',*ROOT.glob('src/**/*.rs')]):
    fingerprint.update(str(source.relative_to(ROOT)).encode()+b'\0'+source.read_bytes()+b'\0')
metadata['rust_source_sha256']=fingerprint.hexdigest()
metadata['measured_at_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat()
results=[];errors=[];rng=random.Random(20261027)
for trial in range(args.rounds):
    cases=list(args.cases);rng.shuffle(cases)
    for case in cases:
        languages=['rust','robotjs']
        if trial%2: languages.reverse()
        for language in languages:
            command=([str(ROOT/'target/release/examples/benchmark'),str(fixtures),case] if language=='rust'
                     else ['node',str(ROOT/'scripts/benchmark.cjs'),str(robot),str(fixtures),case])
            completed=subprocess.run(command,cwd=ROOT,text=True,capture_output=True,timeout=180)
            log=output/f'{trial+1}-{case}-{language}.log'
            log.write_text(completed.stdout+'\n'+completed.stderr)
            if completed.returncode:
                errors.append({'round':trial+1,'case':case,'language':language,'log':str(log),'exit_code':completed.returncode})
                print('FAILED',case,language,log,flush=True)
                continue
            result=json.loads(completed.stdout.strip().splitlines()[-1]);result['round']=trial+1
            results.append(result)
            print(f"{trial+1}/{args.rounds} {case:23} {language:7} {statistics.median(result['ns_per_op'])/1000:12.3f} us/op",flush=True)
        (output/'raw.json').write_text(json.dumps({'metadata':metadata,'results':results,'errors':errors},indent=2)+'\n')

metadata['saved_file_bytes']={p.name:p.stat().st_size for p in fixtures.glob('*-output.*')}
rows=[]
for case in args.cases:
    groups={lang:[r for r in results if r['case']==case and r['language']==lang] for lang in ['rust','robotjs']}
    if any(len(group)!=args.rounds for group in groups.values()):continue
    if case not in ['get_mouse_pos','get_pixel_color']:
        assert len({r['observed'] for group in groups.values() for r in group})==1,f'cross-language result mismatch: {case}'
    row={'case':case}
    for language,group in groups.items():
        samples=[v for r in group for v in r['ns_per_op']]
        row[language+'_us']=statistics.median(samples)/1000
        row[language+'_min_round_us']=min(statistics.median(r['ns_per_op']) for r in group)/1000
        row[language+'_max_round_us']=max(statistics.median(r['ns_per_op']) for r in group)/1000
    row['robotjs_over_rust']=row['robotjs_us']/row['rust_us'];rows.append(row)
(output/'raw.json').write_text(json.dumps({'metadata':metadata,'results':results,'errors':errors},indent=2)+'\n')
with (output/'summary.csv').open('w',newline='') as f:
    if rows:
        writer=csv.DictWriter(f,fieldnames=rows[0].keys());writer.writeheader();writer.writerows(rows)
lines=['# RobotJS vs rusty-robot benchmark','',
       'Median microseconds per operation. Ratio = RobotJS / Rust; above 1 favors Rust.',
       '', '| Operation | Rust µs | RobotJS µs | Ratio |','| --- | ---: | ---: | ---: |']
for row in rows:lines.append(f"| {row['case']} | {row['rust_us']:.3f} | {row['robotjs_us']:.3f} | {row['robotjs_over_rust']:.2f}× |")
lines+=['','```json',json.dumps(metadata,indent=2),'```','',f'Failed measurements: {len(errors)}. Raw samples: raw.json.']
(output/'summary.md').write_text('\n'.join(lines)+'\n')
print('Results:',output/'summary.md',flush=True)
if errors:raise SystemExit(1)
