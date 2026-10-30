#!/usr/bin/env python3
"""Check app captures by independent raw-pixel scanning and DOM geometry."""
import argparse
import array
import hashlib
import json
import sys
from pathlib import Path

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('folder',type=Path)
parser.add_argument('--check',action='store_true')
args=parser.parse_args()
fixtures=json.loads((args.folder/'fixtures.json').read_text())
assert len(fixtures)==60
adjusted=[]
for f in fixtures:
    raw=(args.folder/(f['name']+'.bgra')).read_bytes()
    assert hashlib.sha256(raw).hexdigest()==f['sha256']
    assert hashlib.sha256((args.folder/(f['name']+'.png')).read_bytes()).hexdigest()==f['png_sha256']
    pixels=array.array('I');pixels.frombytes(raw)
    if sys.byteorder!='little':pixels.byteswap()
    colors={int(c,16) for c in f['colors']}
    x0,y0,w,h=f['roi'];stride=f['byte_width']//4
    reference=[]
    for y in range(y0,y0+h):
        xs=[x for x in range(x0,x0+w) if pixels[y*stride+x]&0xffffff in colors]
        if not xs:continue
        if reference and reference[-1][1]+reference[-1][3]==y:
            r=reference[-1];end=max(r[0]+r[2],xs[-1]+1);r[0]=min(r[0],xs[0]);r[2]=end-r[0];r[3]+=1
        else:reference.append([xs[0],y,xs[-1]-xs[0]+1,1])
    dom=f.get('dom_expected',f['expected'])
    assert len(reference)==len(dom),(f['name'],'DOM region count',reference,dom)
    for actual,expected in zip(reference,dom):
        # Collapsed table borders can paint one pixel outside a DOM cell's box.
        edges=lambda r:[r[0],r[1],r[0]+r[2],r[1]+r[3]]
        assert all(abs(a-b)<=1 for a,b in zip(edges(actual),edges(expected))),(f['name'],'DOM geometry',actual,expected)
    if reference!=dom:adjusted.append(f['name'])
    if args.check:
        assert reference==f['expected'],(f['name'],'raw-pixel reference')
        assert f['oracle_verified']
    else:
        f.update(dom_expected=dom,expected=reference,oracle_verified=True)
if not args.check:
    (args.folder/'fixtures.json').write_text(json.dumps(fixtures,indent=2)+'\n')
    capture=json.loads((args.folder/'capture.json').read_text())
    capture['pixel_oracle']={'method':'Independent Python scan of raw BGRA bytes. Exact output rectangles retained; each must also match application DOM band count and all four edges within one pixel for rasterized table borders. Original DOM rectangles retained.','source_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'dom_border_adjustment_frames':adjusted,'verified_frames':60}
    (args.folder/'capture.json').write_text(json.dumps(capture,indent=2)+'\n')
print(f'{args.folder.name}: all 60 raw-pixel references agree with app geometry; {len(adjusted)} frames have a one-pixel border difference.')
