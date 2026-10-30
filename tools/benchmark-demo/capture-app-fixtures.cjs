// Capture an owned VS Code or Chrome app window through Rusty Robot.
// See README for isolated profiles, source inputs, and launch commands.
const fs = require("node:fs"),
  path = require("node:path"),
  crypto = require("node:crypto");
const assert = require("node:assert/strict");
const { execFileSync, spawn } = require("node:child_process");
const { createInterface } = require("node:readline");
const { connect, sleep } = require("./app-cdp.cjs");
const [app, output, pidArg] = process.argv.slice(2),
  pid = Number(pidArg);
assert(["vscode", "github"].includes(app) && Number.isInteger(pid) && pid > 0);
assert(!fs.existsSync(output), "Use a fresh output directory");
fs.mkdirSync(output, { recursive: true });
const native = path.join(__dirname, "target/release/app-capture");
const sha = (file) =>
  crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
const guard = (activate = false) =>
  execFileSync(
    path.join(__dirname, "target/release/owned-window"),
    [
      String(pid),
      "360",
      "140",
      "1200",
      "800",
      ...(activate ? ["--activate"] : []),
    ],
    { encoding: "utf8" },
  );
function expected(rectangles, roi) {
  const [x, y, w, h] = roi;
  const rects = rectangles
    .map((r) => [
      Math.max(x, Math.round(r.x)),
      Math.max(y, Math.round(r.y)),
      Math.min(x + w, Math.round(r.x + r.width)),
      Math.min(y + h, Math.round(r.y + r.height)),
    ])
    .filter((r) => r[2] > r[0] && r[3] > r[1])
    .sort((a, b) => a[1] - b[1] || a[0] - b[0]);
  const bands = [];
  for (const r of rects) {
    const last = bands.at(-1);
    if (last && r[1] <= last[3]) {
      last[0] = Math.min(last[0], r[0]);
      last[2] = Math.max(last[2], r[2]);
      last[3] = Math.max(last[3], r[3]);
    } else bands.push(r.slice());
  }
  return bands.map((r) => [r[0], r[1], r[2] - r[0], r[3] - r[1]]);
}
(async () => {
  const c = await connect(app === "vscode" ? 9450 : 9451, (t) =>
    app === "vscode"
      ? t.url.startsWith("vscode-file:")
      : t.url.startsWith(
          "https://github.com/priyankark/rusty-robot/pull/24/files",
        ),
  );
  const captureProcess = spawn(native, ["--stream"], {
    stdio: ["pipe", "pipe", "inherit"],
  });
  let resolveCapture, rejectCapture;
  createInterface({ input: captureProcess.stdout }).on("line", (line) =>
    resolveCapture(JSON.parse(line)),
  );
  captureProcess.on("error", (error) => rejectCapture?.(error));
  captureProcess.on("exit", (code) => {
    if (code) rejectCapture?.(Error("Capture process exited " + code));
  });
  const capture = (request) =>
    new Promise((resolve, reject) => {
      resolveCapture = resolve;
      rejectCapture = reject;
      captureProcess.stdin.write(JSON.stringify(request) + "\n");
    });
  const failures = [];
  try {
    await c.send("Page.bringToFront");
    guard(true);
    const fixtures = [];
    const colors = app === "vscode" ? ["9f6a04", "6b351a"] : ["152a1e"];
    for (let i = 0; i < 60; i++) {
      let query = null;
      if (app === "vscode") {
        query = ["let", "return", "self", "pub", "fn", "match"][
          Math.floor(i / 10)
        ];
        if (i % 10 === 0) {
          await c.ev(
            `(()=>{const e=document.querySelector('textarea[aria-label="Find"]');if(!e)throw Error('Open the editor Find widget first');e.focus();e.select()})()`,
          );
          await c.send("Input.insertText", { text: query });
          assert.equal(
            await c.ev(
              `document.querySelector('textarea[aria-label="Find"]').value`,
            ),
            query,
            "Search query was not delivered",
          );
          await c.send("Input.dispatchMouseEvent", {
            type: "mouseWheel",
            x: 500,
            y: 500,
            deltaX: 0,
            deltaY: -200000,
          });
        } else
          await c.send("Input.dispatchMouseEvent", {
            type: "mouseWheel",
            x: 500,
            y: 500,
            deltaX: 0,
            deltaY: 360,
          });
      } else await c.ev(`scrollTo(0,${500 + i * 420})`);
      await sleep(220);
      const meta = await c.ev(
        `({title:document.title,url:location.href,screenX,screenY,outerWidth,outerHeight,innerHeight,innerWidth,scrollY,dpr:devicePixelRatio,rectangles:Array.from(document.querySelectorAll('${app === "vscode" ? ".findMatch,.currentFindMatch" : ".blob-code-addition"}')).map(e=>{const r=e.getBoundingClientRect();return{x:r.x,y:r.y+outerHeight-innerHeight,width:r.width,height:r.height}})})`,
      );
      assert.equal(meta.outerWidth, 1200);
      assert.equal(meta.outerHeight, 800);
      assert.equal(meta.innerWidth, 1200);
      const roi = app === "vscode" ? [84, 150, 790, 580] : [445, 150, 722, 600];
      meta.rectangles = meta.rectangles.filter(
        (r) =>
          r.x < roi[0] + roi[2] &&
          r.x + r.width > roi[0] &&
          r.y < roi[1] + roi[3] &&
          r.y + r.height > roi[1],
      );
      const name = String(i).padStart(2, "0");
      guard();
      const request = {
        prefix: path.join(output, name),
        rect: [meta.screenX, meta.screenY, 1200, 800],
      };
      let bitmap = await capture(request);
      if (bitmap.error) {
        failures.push({ name, error: bitmap.error });
        await sleep(500);
        guard();
        bitmap = await capture(request);
      }
      assert(!bitmap.error, bitmap.error);
      guard();
      assert.equal(bitmap.width, 1200);
      assert.equal(bitmap.height, 800);
      assert.equal(bitmap.scale_x, 1);
      assert.equal(bitmap.scale_y, 1);
      fixtures.push({
        name,
        app,
        query,
        ...bitmap,
        colors,
        roi,
        expected: expected(meta.rectangles, roi),
        dom: meta,
        sha256: sha(path.join(output, name + ".bgra")),
        png_sha256: sha(path.join(output, name + ".png")),
      });
      fs.writeFileSync(
        path.join(output, "capture-progress.json"),
        JSON.stringify({ fixtures, failures }, null, 2) + "\n",
      );
      console.log(app, name, fixtures.at(-1).expected.length);
    }
    assert.equal(
      new Set(fixtures.map((f) => f.sha256)).size,
      60,
      "Duplicate captured frame",
    );
    assert(
      fixtures.reduce((n, f) => n + f.expected.length, 0) > 20,
      "Corpus has too few visible target regions",
    );
    fs.writeFileSync(
      path.join(output, "fixtures.json"),
      JSON.stringify(fixtures, null, 2) + "\n",
    );
    fs.writeFileSync(
      path.join(output, "capture.json"),
      JSON.stringify(
        {
          app,
          failures,
          created_at: new Date().toISOString(),
          capture:
            "Rusty Robot Robot.capture(Some(Rect)); PNG saved by Rust; capture excluded from analysis timings",
          capture_binary_sha256: sha(native),
          capture_source_sha256: sha(
            path.join(__dirname, "src/bin/app-capture.rs"),
          ),
          collector_sha256: sha(__filename),
          oracle:
            "Expected vertical bands formed from clipped application DOM highlight rectangles, independent of either image-search implementation",
          app_version: execFileSync(
            "/usr/libexec/PlistBuddy",
            [
              "-c",
              "Print :CFBundleShortVersionString",
              app === "vscode"
                ? "/Applications/Visual Studio Code.app/Contents/Info.plist"
                : "/Applications/Google Chrome.app/Contents/Info.plist",
            ],
            { encoding: "utf8" },
          ).trim(),
        },
        null,
        2,
      ) + "\n",
    );
  } finally {
    captureProcess.stdin.end();
    c.ws.close();
  }
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
