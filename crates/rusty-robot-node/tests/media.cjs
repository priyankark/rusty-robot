const assert = require('node:assert/strict');
const { test } = require('node:test');
const { deflateSync, crc32 } = require('node:zlib');
const native = require('../../../target/rusty-robot.node');
// An owned opaque red fixture, generated without an image dependency.
function png(width, height) {
  const chunk = (name, bytes) => { const type = Buffer.from(name), out = Buffer.alloc(bytes.length + 12); out.writeUInt32BE(bytes.length); type.copy(out, 4); bytes.copy(out, 8); out.writeUInt32BE(crc32(Buffer.concat([type, bytes])), out.length - 4); return out; };
  const header = Buffer.alloc(13); header.writeUInt32BE(width); header.writeUInt32BE(height,4); header[8]=8; header[9]=2;
  const raw = Buffer.alloc((width*3+1)*height); for(let y=0;y<height;y++)for(let x=0;x<width;x++)raw[y*(width*3+1)+1+x*3]=255;
  return Buffer.concat([Buffer.from('89504e470d0a1a0a','hex'), chunk('IHDR',header), chunk('IDAT',deflateSync(raw)), chunk('IEND',Buffer.alloc(0))]);
}
test('asynchronous media validates, owns input, bounds the queue, releases permits and preserves formats', async () => {
  const fixture = png(1920,1080);
  for (const value of [NaN, Infinity, -1, 0, 0.5, 16385]) assert.throws(()=>native.resizeImage(fixture,value), /integer|dimension/i);
  assert.throws(()=>native.captureFrame(800,101), /quality/i);
  assert.throws(()=>native.captureFrame(800,85,null,true,'gif'), /format/i);
  let ticks=0; const timer=setInterval(()=>ticks++,1);
  try {
    const source = Buffer.from(fixture);
    const first = native.resizeImage(source,1440,null,85,'jpeg');
    source.fill(0); // Mutation after scheduling must not race native decode.
    const second = native.resizeImage(fixture,800,null,85,'png');
    assert.throws(()=>native.resizeImage(fixture,800), /queue is full/);
    const [jpeg,pngResult]=await Promise.all([first,second]);
    assert.equal(jpeg.data.subarray(0,2).toString('hex'),'ffd8');
    assert.equal(pngResult.data.subarray(0,8).toString('hex'),'89504e470d0a1a0a');
    assert.deepEqual([jpeg.width,jpeg.height],[1440,810]);
    assert.deepEqual([pngResult.width,pngResult.height],[800,450]);
    assert(ticks > 0, 'JS timers must progress while image workers run');
    await assert.rejects(native.resizeImage(Buffer.from('broken'),800), /JPEG|PNG|image|format/i);
    const again=await native.resizeImage(fixture,1440,null,85,'jpeg');
    assert.equal(again.fingerprint,jpeg.fingerprint, 'permit released and deterministic pixels');
  } finally { clearInterval(timer); }
});
test('real desktop capture is asynchronous and returns a valid sized frame', {skip: !process.env.RUSTY_ROBOT_TEST_CAPTURE}, async () => {
  let ticks=0; const timer=setInterval(()=>ticks++,1);
  try {
    const frame=await native.captureFrame(1440,85,null,true,'jpeg');
    assert(frame.width > 0 && frame.width <= 1440 && frame.height > 0);
    assert.equal(frame.data.subarray(0,2).toString('hex'),'ffd8'); assert(ticks > 0);
    const png=await native.captureFrame(800,85,null,true,'png');
    assert.equal(png.data.subarray(0,8).toString('hex'),'89504e470d0a1a0a');
  } finally { clearInterval(timer); }
});
