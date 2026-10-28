#!/usr/bin/env python3
"""Live capture correctness; no keyboard input and no screenshots saved.
Optional cursor test moves only over the owned fixture and restores the pointer.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time
from run import Process, HERE, BINARY

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('output',type=Path)
    parser.add_argument('--baseline',type=Path)
    parser.add_argument('--cursor',action='store_true')
    args=parser.parse_args();args.output.parent.mkdir(parents=True,exist_ok=True)
    target_path=HERE/'target/capture-quality-target'
    subprocess.run(['swiftc',str(HERE/'target.swift'),'-o',str(target_path)],check=True)
    target=Process([target_path],'/tmp/rust-capture-quality-target.log')
    driver=None;baseline=None;records=[];cursor_result='not requested';cursor=None;passed=False
    try:
        driver=Process([BINARY,'rusty-robot'],'/tmp/rust-capture-quality-driver.log')
        if args.baseline:baseline=Process([args.baseline,'rusty-robot'],'/tmp/rust-capture-quality-baseline.log')
        g=target.ready['patch'];cx=g['x']+g['width']//2;cy=g['y']+g['height']//2
        colors=[[255,0,0],[0,255,0],[0,0,255],[192,255,51],[17,34,51],[64,128,192],
                [128,128,128],[255,255,255],[0,0,0],[255,128,0],[0,255,255],[255,0,255]]
        def capture(x,y,w,h,points,full=False):
            result=driver.ask({'op':'capture_check','x':x,'y':y,'width':w,'height':h,'full':full,'probes':points})
            assert result.get('ok'), result
            assert (result['width'],result['height'])==(w,h),result
            assert result['screen_x']==x and result['screen_y']==y,result
            assert result['scale_x']==1 and result['scale_y']==1,result
            assert all(p['alpha']==255 for p in result['probes']),result
            return [[int(p['rgb'][i:i+2],16) for i in (0,2,4)] for p in result['probes']]
        display=driver.ready['display']
        for expected in colors:
            target.ask({'rgb':expected});time.sleep(.08)
            points=[[g['x'],g['y']],[g['x']+g['width']-1,g['y']],
                    [g['x'],g['y']+g['height']-1],[g['x']+g['width']-1,g['y']+g['height']-1],[cx,cy]]
            full=capture(0,0,display['width'],display['height'],points,True)
            small=[capture(x,y,1,1,[[0,0]])[0] for x,y in points]
            assert small==full,{'expected':expected,'full':full,'one_pixel':small}
            row={'kind':'palette','expected_srgb':expected,'full':full,'one_pixel':small}
            if baseline:
                old=baseline.ask({'op':'capture','x':cx,'y':cy,'width':1,'height':1,'probes':[[0,0]]})
                assert old.get('ok'),old
                row['baseline_center']=old['probes'][0]
                delta=max(abs(a-b) for a,b in zip(small[-1],old['probes'][0]))
                assert delta<=3, row
                row['max_channel_delta_from_baseline']=delta
            records.append(row)
        target.ask({'checkerboard':True});time.sleep(.08)
        points=[[x,y] for y in [0,1,2,29,58,59] for x in [0,1,2,99,198,199]]
        full=capture(0,0,display['width'],display['height'],[[g['x']+x,g['y']+y] for x,y in points],True)
        region=capture(g['x'],g['y'],g['width'],g['height'],points)
        small=[capture(g['x']+x,g['y']+y,1,1,[[0,0]])[0] for x,y in points]
        expected=[[255,255,255] if (x+y)%2==0 else [0,0,0] for x,y in points]
        assert full==region==small==expected,{'full':full,'region':region,'small':small,'expected':expected}
        records.append({'kind':'checkerboard','points':points,'expected':expected,'full':full,'region':region,'one_pixel':small})
        if args.cursor:
            state=target.ask({'rgb':[128,128,128]});time.sleep(.08)
            if not state.get('active'):cursor_result='skipped: owned window not active'
            else:
                cursor=driver.ready['cursor']
                assert driver.ask({'op':'move','x':cx,'y':cy}).get('ok')
                time.sleep(.04)
                samples=capture(cx-5,cy-5,30,30,[[x,y] for y in range(30) for x in range(30)])
                assert all(p==[128,128,128] for p in samples),f'Unexpected cursor-region pixels: {sorted(set(tuple(p) for p in samples))[:12]}'
                cursor_result='passed: all 900 pixels unchanged with pointer over patch'
        for x,y in [(0,0),(display['width']-1,0),(0,display['height']-1),(display['width']-1,display['height']-1)]:
            # Bounds/opacity only at desktop corners; never persist their RGB values.
            capture(x,y,1,1,[[0,0]])
        records.append({'kind':'display_corners','opaque':4})
        passed=True
        print('Capture quality checks passed')
    finally:
        if cursor and driver:driver.ask({'op':'move',**cursor})
        if baseline:baseline.close()
        if driver:driver.close()
        target.close()
        args.output.write_text(json.dumps({'passed':passed,'records':records,'cursor_test':cursor_result,
            'binary_sha256':hashlib.sha256(BINARY.read_bytes()).hexdigest()},indent=2)+'\n')
if __name__=='__main__':main()
