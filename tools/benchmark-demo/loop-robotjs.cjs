const robot = require(require("node:path").resolve(process.argv[2]));
const assert = require("node:assert/strict");
const matches = (a, b) =>
  [0, 2, 4].every(
    (i) =>
      Math.abs(
        parseInt(a.slice(i, i + 2), 16) - parseInt(b.slice(i, i + 2), 16),
      ) <= 1,
  );
const now = () => Number(process.hrtime.bigint()) / 1e6;
function run(v) {
  if (v.op === "probe")
    return {
      accessibility: robot.getAccessibilityPermission(),
      capture: robot.getScreenCapturePermission(),
      displays: robot.getDisplays(),
      pointer: robot.getMousePos(),
    };
  robot.setMouseDelay(v.delay || 0);
  const start = now(),
    b = robot.screen.capture(...v.rect),
    capture_ms = now() - start;
  assert.equal(b.width, v.rect[2]);
  assert.equal(b.height, v.rect[3]);
  assert.equal(b.scaleX, 1);
  assert.equal(b.scaleY, 1);
  const actual_before = b.colorAt(...v.sample);
  assert(matches(actual_before, v.before), "Stale or incorrect first capture");
  const result = {
    capture_ms,
    width: b.width,
    height: b.height,
    scale_x: b.scaleX,
    scale_y: b.scaleY,
    first_color: actual_before,
  };
  if (v.op === "loop") {
    const search = now(),
      p = b.findColor("ff00ff");
    assert(p, "Target missing");
    const q = b.toScreenPoint(p),
      find_ms = now() - search,
      x = q.x + 10,
      y = q.y + 10;
    assert(
      x >= v.rect[0] &&
        y >= v.rect[1] &&
        x < v.rect[0] + v.rect[2] &&
        y < v.rect[1] + v.rect[3],
    );
    const input = now();
    robot.moveMouse(x, y);
    robot.mouseClick();
    const input_ms = now() - input;
    const observe = now();
    let polls = 0,
      actual_after,
      complete = true;
    for (;;) {
      const fresh = robot.screen.capture(...v.rect);
      polls++;
      const color = fresh.colorAt(...v.sample);
      if (matches(color, v.after)) {
        actual_after = color;
        break;
      }
      if (now() - start > 1000) {
        actual_after = color;
        complete = false;
        break;
      }
    }
    const observe_ms = now() - observe,
      total_ms = now() - start;
    result.loop = {
      find_ms,
      input_ms,
      observe_ms,
      total_ms,
      polls,
      click: [x, y],
      last_color: actual_after,
      complete,
    };
  }
  return result;
}
require("node:readline")
  .createInterface({ input: process.stdin })
  .on("line", (line) => {
    try {
      console.log(JSON.stringify({ ok: true, result: run(JSON.parse(line)) }));
    } catch (e) {
      console.log(JSON.stringify({ ok: false, error: e.message }));
    }
  });
