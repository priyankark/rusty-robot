// Encoded-image processing only: identical owned PNG bytes, dimensions and JPEG quality.
// Dependencies are supplied explicitly from the consumer under test; no hidden installs.
const { performance } = require('node:perf_hooks');
const { deflateSync, crc32 } = require('node:zlib');
const { writeFileSync } = require('node:fs');
const assert = require('node:assert/strict');
const { Jimp, ResizeStrategy } = require(process.env.BENCHMARK_JIMP);
const native = require(process.env.BENCHMARK_ADDON);
const backend=process.env.BENCHMARK_BACKEND;
function fixture(width,height) {
  const rgb=Buffer.alloc(width*height*3),raw=Buffer.alloc((width*3+1)*height);
  for(let y=0;y<height;y++)for(let x=0;x<width;x++){
    const cell=(Math.floor(x/64)+Math.floor(y/48))%2;
    const color=[(x*255/(width-1))|0,(y*255/(height-1))|0,cell?210:40];
    for(let k=0;k<3;k++){rgb[(y*width+x)*3+k]=color[k];raw[y*(width*3+1)+1+x*3+k]=color[k];}
  }
  const chunk=(name,bytes)=>{const type=Buffer.from(name),out=Buffer.alloc(bytes.length+12);out.writeUInt32BE(bytes.length);type.copy(out,4);bytes.copy(out,8);out.writeUInt32BE(crc32(Buffer.concat([type,bytes])),out.length-4);return out;};
  const head=Buffer.alloc(13);head.writeUInt32BE(width);head.writeUInt32BE(height,4);head[8]=8;head[9]=2;
  return {rgb,png:Buffer.concat([Buffer.from('89504e470d0a1a0a','hex'),chunk('IHDR',head),chunk('IDAT',deflateSync(raw)),chunk('IEND',Buffer.alloc(0))])};
}
// Independent area-average reference, computed outside measurement.
function reference(rgb,sw,sh,dw,dh) {
  const out=Buffer.alloc(dw*dh*3),sx=sw/dw,sy=sh/dh;
  for(let y=0;y<dh;y++)for(let x=0;x<dw;x++){
    const x0=x*sx,x1=(x+1)*sx,y0=y*sy,y1=(y+1)*sy,sum=[0,0,0];
    for(let j=Math.floor(y0);j<Math.ceil(y1);j++)for(let i=Math.floor(x0);i<Math.ceil(x1);i++){
      const weight=(Math.min(x1,i+1)-Math.max(x0,i))*(Math.min(y1,j+1)-Math.max(y0,j));
      for(let k=0;k<3;k++)sum[k]+=rgb[(j*sw+i)*3+k]*weight;
    }
    for(let k=0;k<3;k++)out[(y*dw+x)*3+k]=Math.round(sum[k]/sx/sy);
  }return out;
}
(async()=>{
  const cases=[];
  for(const [name,width,height] of [['decode_encode',1440,810],['decode_resize_encode',1920,1080]]){
    const f=fixture(width,height),expected=reference(f.rgb,width,height,1440,810),samples=[];
    let last;
    for(let i=-1;i<15;i++){
      const cpu=process.cpuUsage(),start=performance.now();let stages=null;
      if(backend==='rusty-robot') {const frame=await native.resizeImage(f.png,1440,810,85,'jpeg');last=frame.data;stages={resize_ms:frame.resizeMs,hash_ms:frame.hashMs,encode_ms:frame.encodeMs};}
      else {const image=await Jimp.read(f.png);if(width!==1440)image.resize({w:1440,h:810,mode:ResizeStrategy.BILINEAR});last=await image.getBuffer('image/jpeg',{quality:85});}
      const elapsed=performance.now()-start,used=process.cpuUsage(cpu);
      if(i>=0)samples.push({ms:elapsed,cpu_ms:(used.user+used.system)/1000,bytes:last.length,stages});
    }
    const decoded=await Jimp.read(last);assert.equal(decoded.width,1440);assert.equal(decoded.height,810);
    let error=0;for(let i=0;i<1440*810;i++)for(let k=0;k<3;k++)error+=(decoded.bitmap.data[i*4+k]-expected[i*3+k])**2;
    const mse=error/(1440*810*3),psnr=10*Math.log10(255**2/mse);
    cases.push({name,source_width:width,source_height:height,samples,psnr_db:psnr,correctness:{decoded:true,width:1440,height:810}});
  }
  writeFileSync(process.argv[2],JSON.stringify({backend,node:process.version,jimp:require(process.env.BENCHMARK_JIMP+'/package.json').version,quality:85,warmups:1,repetitions:15,cases},null,2));
})().catch(e=>{console.error(e);process.exitCode=1});
