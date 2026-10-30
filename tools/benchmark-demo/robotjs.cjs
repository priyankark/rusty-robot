const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const [robotPath,root,name,n]=process.argv.slice(2),total=Number(n);
assert(Number.isInteger(total)&&total>=100&&total<=100000);
const robot=require(path.resolve(robotPath));
const bitmap=(file,w,h)=>new robot.Bitmap(w,h,w*4,32,4,fs.readFileSync(path.join(root,file)));
const canvas=bitmap('canvas.bgra',1920,1080),dense=bitmap('dense.bgra',256,256),needle=bitmap('needle.bgra',32,32);
const tasks={
 find_colors_dense:()=>dense.findColors('334455').length,
 find_color_absent:()=>Number(Boolean(canvas.findColor('00ffff'))),
 find_image_late:()=>{const p=canvas.findImage(needle);return (p.x+1)*4096+p.y+1},
 load_bmp:()=>{const b=robot.image.load(path.join(root,'source.bmp'));assert.equal(b.colorAt(123,456),canvas.colorAt(123,456));return b.width*b.height}
};
const expected={find_colors_dense:65536,find_color_absent:0,find_image_late:1857*4096+1017,load_bmp:1920*1080}[name];
assert(expected!==undefined);for(let i=0;i<5;i++)assert.equal(tasks[name](),expected);
const emit=x=>fs.writeSync(1,JSON.stringify(x)+'\n');
emit({type:'ready'});
process.stdin.once('data',data=>{
 assert.equal(data.toString().trim(),'go');const start=process.hrtime.bigint();
 for(let i=1;i<=total;i++){
  assert.equal(tasks[name](),expected);
  if(i%Math.floor(total/100)===0||i===total)emit({type:'progress',done:i,total,seconds:Number(process.hrtime.bigint()-start)/1e9,result:expected,verified:true});
 }
 emit({type:'complete',done:total,total,seconds:Number(process.hrtime.bigint()-start)/1e9,result:expected,verified:true});process.stdin.destroy();
});
