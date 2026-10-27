// RobotJS public API, including the real native addon. No synthetic JS replacement.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const [robotPath, root, name] = process.argv.slice(2);
const robot = require(path.resolve(robotPath));
assert.equal(robot.image.supportsPNG, true);
const bitmap = (file,w,h) => new robot.Bitmap(w,h,w*4,32,4,fs.readFileSync(path.join(root,file)));
const canvas=bitmap('canvas.bgra',1920,1080), needle=bitmap('needle.bgra',32,32);
const absent=bitmap('absent.bgra',32,32), dense=bitmap('dense.bgra',256,256), tile=bitmap('tile.bgra',4,4);
const bmp=path.join(root,'source.bmp'), png=path.join(root,'source.png');
const outBmp=path.join(root,'js-output.bmp'), outPng=path.join(root,'js-output.png');
const point=p=>p ? ((p.x+1)*4096+p.y+1)>>>0 : 0;
const functions={
  color_at:()=>parseInt(canvas.colorAt(123,456),16),
  find_color_early:()=>point(canvas.findColor('000000')),
  find_color_late:()=>point(canvas.findColor('ff00ff')),
  find_color_absent:()=>point(canvas.findColor('00ffff')),
  count_color_absent:()=>canvas.countColor('00ffff'),
  count_color_dense:()=>dense.countColor('334455'),
  find_colors_dense:()=>dense.findColors('334455').length,
  find_image_late:()=>point(canvas.findImage(needle)),
  find_image_absent:()=>point(canvas.findImage(absent)),
  find_image_tolerant:()=>point(canvas.findImage(needle,{tolerance:0.02})),
  count_image_dense:()=>dense.countImage(tile),
  find_images_dense:()=>dense.findImages(tile).length,
  load_bmp:()=>{const image=robot.image.load(bmp);return image.width*image.height;},
  load_png:()=>{const image=robot.image.load(png);return image.width*image.height;},
  save_bmp:()=>{canvas.save(outBmp);return 0;},
  save_png:()=>{canvas.save(outPng);return 0;},
  get_mouse_pos:()=>point(robot.getMousePos()),
  get_displays:()=>robot.getDisplays().length,
  get_pixel_color:()=>parseInt(robot.getPixelColor(0,0),16),
  capture_small:()=>{const image=robot.screen.capture(0,0,100,100);return image.width*image.height;},
  capture_full:()=>{const image=robot.screen.capture();return image.width*image.height;},
};
const f=functions[name];assert.ok(f,`unknown benchmark ${name}`);
const observed=f();
const expected={color_at:(((123*3+456*5)%128)<<16)|(((123*7+456*13)%128)<<8)|((123*17+456*11)%128),
 find_color_early:4097,find_color_late:1857*4096+1017,find_image_late:1857*4096+1017,find_image_tolerant:1857*4096+1017,
 find_color_absent:0,count_color_absent:0,find_image_absent:0,save_bmp:0,save_png:0,
 count_color_dense:256*256,find_colors_dense:256*256,count_image_dense:253*253,find_images_dense:253*253,
 load_bmp:1920*1080,load_png:1920*1080,capture_small:100*100};
if(name in expected) assert.equal(observed,expected[name],`incorrect result for ${name}`);
if(name.startsWith('load_')||name.startsWith('save_')){
 const file={load_bmp:bmp,load_png:png,save_bmp:outBmp,save_png:outPng}[name];
 const image=robot.image.load(file);assert.equal(image.width,1920);assert.equal(image.height,1080);
 for(const [x,y] of [[0,0],[123,456],[1856,1016],[1887,1047],[1919,1079]])assert.equal(image.colorAt(x,y),canvas.colorAt(x,y));
}
function batch(n){const t=process.hrtime.bigint();let checksum=0;for(let i=0;i<n;i++)checksum=(checksum+f())>>>0;return [Number(process.hrtime.bigint()-t),checksum];}
for(let i=0;i<5;i++)f();
let n=1;while(batch(n)[0]<40e6&&n<262144)n*=2;
const samples=[];let checksum=0;
for(let i=0;i<7;i++){const [ns,c]=batch(n);samples.push(ns/n);checksum^=c;}
console.log(JSON.stringify({language:'robotjs',case:name,iterations:n,ns_per_op:samples,observed,checksum:checksum>>>0}));
