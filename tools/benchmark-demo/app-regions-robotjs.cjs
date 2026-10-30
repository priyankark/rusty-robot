const fs = require("node:fs");
const path = require("node:path");
const assert = require("node:assert/strict");
const [robotPath, folder, mode] = process.argv.slice(2);
const robot = require(path.resolve(robotPath));
const fixtures = JSON.parse(
  fs.readFileSync(path.join(folder, "fixtures.json")),
);
const images = fixtures.map(
  (f) =>
    new robot.Bitmap(
      f.width,
      f.height,
      f.byte_width,
      32,
      4,
      fs.readFileSync(path.join(folder, f.name + ".bgra")),
    ),
);
function regions(image, f) {
  const rows = Array.from({ length: f.height }, () => [f.width, -1]);
  for (const color of f.colors) {
    for (const p of image.findColors(color, {
      x: f.roi[0],
      y: f.roi[1],
      width: f.roi[2],
      height: f.roi[3],
    })) {
      rows[p.y][0] = Math.min(rows[p.y][0], p.x);
      rows[p.y][1] = Math.max(rows[p.y][1], p.x);
    }
  }
  const out = [];
  for (let y = 0; y < rows.length; y++) {
    const [left, right] = rows[y];
    if (right < 0) continue;
    const last = out.at(-1);
    if (last && last[1] + last[3] === y) {
      const end = Math.max(last[0] + last[2], right + 1);
      last[0] = Math.min(last[0], left);
      last[2] = end - last[0];
      last[3]++;
    } else out.push([left, y, right - left + 1, 1]);
  }
  return out;
}
const run = (i) => regions(images[i], fixtures[i]);
fixtures.forEach((f, i) => assert.deepEqual(run(i), f.expected));
const emit = (value) => fs.writeSync(1, JSON.stringify(value) + "\n");
emit({ type: "ready" });
process.stdin.once("data", (data) => {
  assert.equal(data.toString().trim(), "go");
  const start = process.hrtime.bigint(),
    results = [];
  for (let i = 0; i < fixtures.length; i++) {
    results.push(run(i));
    if (mode === "record")
      emit({
        type: "progress",
        index: i,
        result: results.at(-1),
        seconds: Number(process.hrtime.bigint() - start) / 1e9,
      });
  }
  const seconds = Number(process.hrtime.bigint() - start) / 1e9;
  results.forEach((r, i) => assert.deepEqual(r, fixtures[i].expected));
  emit({
    type: "complete",
    seconds,
    screenshots: fixtures.length,
    targets: results.reduce((n, r) => n + r.length, 0),
    verified: true,
    results,
  });
  process.stdin.destroy();
});
