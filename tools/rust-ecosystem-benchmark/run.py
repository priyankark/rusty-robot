#!/usr/bin/env python3
"""macOS only. Uses an owned AppKit target; never types into another app.
Run from repository root: python3 tools/rust-ecosystem-benchmark/run.py OUTPUT_DIR
Requires an unlocked desktop, Accessibility and Screen Recording permission.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import queue
import statistics
import subprocess
import threading
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
BINARY = HERE / 'target/release/rust-ecosystem-benchmark'

class Process:
    def __init__(self, args, log):
        self.log = open(log, 'w')
        self.p = subprocess.Popen(list(map(str, args)), stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=self.log, text=True, bufsize=1)
        self.q = queue.Queue()
        def read():
            for line in self.p.stdout:
                try: self.q.put(json.loads(line))
                except ValueError: self.q.put({'protocol_error': line})
            self.q.put({'process_exited': self.p.poll()})
        threading.Thread(target=read, daemon=True).start()
        self.ready = self.q.get(timeout=15)
        if not self.ready.get('ready'): raise RuntimeError(self.ready)
    def ask(self, value):
        self.p.stdin.write(json.dumps(value, ensure_ascii=False) + '\n')
        self.p.stdin.flush()
        result = self.q.get(timeout=15)
        if 'process_exited' in result or 'protocol_error' in result: raise RuntimeError(result)
        return result
    def close(self):
        self.p.stdin.close()
        try: self.p.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.p.terminate(); self.p.wait(timeout=5)
        self.log.close()

def checked_state(target, command=None):
    state = target.ask(command or {})
    if not state.get('active'): raise RuntimeError('Owned benchmark target lost focus; stopping input')
    return state

def percentile(values, p):
    v = sorted(values)
    return v[max(0, __import__('math').ceil(len(v)*p)-1)]

def summarize(rows):
    groups = {}
    for row in rows:
        if row.get('warmup'): continue
        key = (row['category'], row['backend'], row.get('profile',''), row['case'])
        groups.setdefault(key, []).append(row)
    output = []
    for key, group in groups.items():
        good = [r for r in group if r['success']]
        result = dict(zip(['category','backend','profile','case'],key))
        result.update(successes=len(good), attempts=len(group))
        for field in ['api_ms','observed_ms','normalized_ms']:
            values = [r[field] for r in good if field in r]
            if not values: continue
            rounds = []
            for round_id in sorted(set(r['round'] for r in good)):
                rv=[r[field] for r in good if r['round']==round_id and field in r]
                if rv: rounds.append(statistics.median(rv))
            result[field]={'median':statistics.median(values), 'p95':percentile(values,.95),
                           'round_medians':rounds, 'min_round_median':min(rounds), 'max_round_median':max(rounds)}
        output.append(result)
    return output

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('output', type=Path)
    parser.add_argument('--rounds',type=int,default=3)
    parser.add_argument('--trials',type=int,default=15)
    parser.add_argument('--capture-only',action='store_true',help='No keyboard or mouse injection')
    args=parser.parse_args()
    out=args.output.resolve(); out.mkdir(parents=True,exist_ok=True)
    fixture=HERE/'target/native-target'
    subprocess.run(['swiftc',str(HERE/'target.swift'),'-o',str(fixture)],check=True)
    rows=[]; metadata={'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        'rustc':subprocess.check_output(['rustc','-Vv'],text=True),
        'os':subprocess.check_output(['sw_vers'],text=True), 'arch':platform.machine(),
        'cpu':subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),
        'memory_bytes':int(subprocess.check_output(['sysctl','-n','hw.memsize'],text=True)),
        'versions':{'rusty-robot-desktop':'0.1.2 source at source_commit','enigo':'0.6.1','xcap':'0.9.8'},
        'rounds':args.rounds,'trials':args.trials,'capture_only':args.capture_only,'warmup_trials':2,'poll_interval_ms':2,
        'timeout_ms':750,'stability_check_ms':20,
        'binary_sha256':hashlib.sha256(BINARY.read_bytes()).hexdigest(),
        'cargo_lock_sha256':hashlib.sha256((HERE/'Cargo.lock').read_bytes()).hexdigest(),
        'background':'Existing desktop processes left running; no other benchmark runs concurrently.'}
    (out/'manifest.json').write_text(json.dumps(metadata,indent=2)+'\n')
    target=None; driver=None; cursor=None
    raw=open(out/'samples.jsonl','w')
    def record(row):
        rows.append(row); raw.write(json.dumps(row,ensure_ascii=False)+'\n'); raw.flush()
    try:
        target=Process([fixture],out/'fixture.stderr.log')
        time.sleep(.3)
        geometry=target.ready
        checked_state(target)
        cases=[('ascii31','text','','The quick brown fox jumps over.'),
               ('unicode','text','','café e\u0301 中文 👩‍💻 🚀'),
               ('emoji32','text','','🚀'*32),
               ('multiline','text','','first line\nsecond line'),
               ('leading_newline','text','','\nsecond line'),
               ('shortcut_replace','replace','replace this text','replacement'),
               ('enter','enter','line','line\n'),
               ('select_delete','select_delete','delete this text',''),
               ('move_click','move_click','',''),('move','move','','')]
        for rnd in range(args.rounds):
            order=[] if args.capture_only else (['rusty-robot','enigo'] if rnd%2==0 else ['enigo','rusty-robot'])
            for backend in order:
                driver=Process([BINARY,backend],out/f'{rnd}-{backend}.stderr.log')
                if not driver.ready['accessibility']: raise RuntimeError('Accessibility permission unavailable')
                cursor=cursor or driver.ready['cursor']; metadata['display']=driver.ready['display']
                for profile in (['default','zero_configured_delay'] if backend=='rusty-robot' else ['default']):
                    driver.ask({'op':'config','delay_ms':10 if profile=='default' else 0})
                    for name,op,initial,expected in cases:
                        for trial in range(args.trials+2):
                            before=checked_state(target,{'reset':initial})
                            time.sleep(.02) # Allow reset/focus to settle, outside timing.
                            checked_state(target)
                            rect=geometry['button' if op=='move_click' else 'editor']
                            x=rect['x']+rect['width']//2+(trial%2)*5
                            y=rect['y']+rect['height']//2
                            command={'op':op,'text':expected,'x':x,'y':y}
                            started=time.perf_counter()
                            result=driver.ask(command)
                            deadline=started+.75
                            success=False
                            while True:
                                state=checked_state(target)
                                if op=='move_click': success=state['clicks']==before['clicks']+1
                                elif op=='move': success=(state['cursorX'],state['cursorY'])==(x,y)
                                else: success=state['text']==expected
                                elapsed=(time.perf_counter()-started)*1000
                                if success or time.perf_counter()>deadline or not result.get('ok'): break
                                time.sleep(.002)
                            if success:
                                time.sleep(.02)
                                stable=checked_state(target)
                                if op=='move_click': success=stable['clicks']==before['clicks']+1
                                elif op=='move': success=(stable['cursorX'],stable['cursorY'])==(x,y)
                                else: success=stable['text']==expected
                                state=stable
                            record({'category':'input','backend':backend,'profile':profile,'case':name,
                                'round':rnd+1,'trial':trial-1,'warmup':trial<2,'success':success and result.get('ok',False),
                                'api_ms':result.get('api_ms'),'observed_ms':elapsed,
                                'expected':expected,'actual':state['text'], 'error':result.get('error')})
                        print(f'round {rnd+1} {backend}/{profile} {name}: done',flush=True)
                    for op,count in [('position',1000),('display_size',100)]:
                        for trial in range(args.trials+2):
                            result=driver.ask({'op':op})
                            record({'category':'query','backend':backend,'profile':profile,'case':op,
                                'round':rnd+1,'trial':trial-1,'warmup':trial<2,'success':result.get('ok',False),
                                'api_ms':result.get('api_ms',0)/count,'calls_per_sample':count})
                driver.close();driver=None
            for backend in (['rusty-robot','xcap'] if rnd%2==0 else ['xcap','rusty-robot']):
                driver=Process([BINARY,backend],out/f'{rnd}-{backend}-capture.stderr.log')
                if not driver.ready['capture_permission']: raise RuntimeError('Screen Recording permission unavailable')
                display=driver.ready['display']; patch=geometry['patch']
                px=patch['x']+patch['width']//2; py=patch['y']+patch['height']//2
                for name,w,h in [('full',display['width'],display['height']),('region100',100,100),('pixel',1,1)]:
                    for trial in range(args.trials+2):
                        green=trial%2==0
                        target.ask({'green':green})
                        time.sleep(.04) # Color presentation settles before timing.
                        full=name=='full'; x=0 if full else px-w//2; y=0 if full else py-h//2
                        probes=[[px-x,py-y]] if name=='pixel' else [[px-x,py-y],[px-x-10 if full else max(0,w//2-10),py-y]]
                        result=driver.ask({'op':'capture','full':full,'width':w,'height':h,'x':x,'y':y,'probes':probes})
                        expected=[0,255,0] if green else [255,0,0]
                        # XCap returns display-space RGB without an ICC profile; Rusty Robot requests sRGB.
                        # Validate fresh red/green content independently of exact sRGB fidelity.
                        success=result.get('ok',False) and all(
                            (p[1]>2*p[0] and p[1]>2*p[2]) if green else (p[0]>2*p[1] and p[0]>2*p[2])
                            for p in result['probes'])
                        exact_srgb=result.get('ok',False) and all(all(abs(a-b)<=3 for a,b in zip(p,expected)) for p in result['probes'])
                        record({'category':'capture','backend':backend,'case':name,'round':rnd+1,
                            'trial':trial-1,'warmup':trial<2,'success':success,'exact_srgb':exact_srgb,'expected_rgb':expected,**result})
                    print(f'round {rnd+1} {backend} capture {name}: done',flush=True)
                driver.close();driver=None
        metadata['completed_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat()
    finally:
        if driver: driver.close()
        if cursor and not args.capture_only:
            restore=Process([BINARY,'rusty-robot'],out/'restore.stderr.log')
            restore.ask({'op':'move',**cursor}); restore.close()
        if target: target.close()
        raw.close()
        (out/'manifest.json').write_text(json.dumps(metadata,indent=2)+'\n')
        (out/'summary.json').write_text(json.dumps(summarize(rows),indent=2)+'\n')

if __name__=='__main__': main()
