const fs = require("node:fs"),
  path = require("node:path"),
  http = require("node:http"),
  crypto = require("node:crypto");
const assert = require("node:assert/strict");
const { spawn, execFileSync } = require("node:child_process");
const { createInterface } = require("node:readline");
const { connect, sleep } = require("./app-cdp.cjs");
const [robotArg, outArg, pilot] = process.argv.slice(2),
  root = path.resolve(__dirname, "../..");
assert(robotArg && outArg, "loop-compare.cjs ROBOTJS NEW_OUTPUT [pilot]");
const robot = path.resolve(robotArg),
  out = path.resolve(outArg),
  tmp = fs.mkdtempSync("/tmp/rusty-loop-");
assert(!fs.existsSync(out), "Use new output directory");
fs.mkdirSync(out, { recursive: true });
const sha = (p) =>
  crypto.createHash("sha256").update(fs.readFileSync(p)).digest("hex");
const sys = (cmd, args) => execFileSync(cmd, args, { encoding: "utf8" }).trim();
let chrome,
  cdp,
  server,
  driver,
  nonce = 0;
function startDriver(language) {
  const p = spawn(
    language === "rust"
      ? path.join(__dirname, "target/release/loop")
      : process.execPath,
    language === "rust"
      ? []
      : [path.join(__dirname, "loop-robotjs.cjs"), robot],
    { stdio: ["pipe", "pipe", "pipe"] },
  );
  let pending,
    stderr = "";
  p.stderr.on("data", (d) => (stderr += d));
  createInterface({ input: p.stdout }).on("line", (line) => {
    if (!pending) return;
    const { resolve, reject, timer } = pending;
    pending = null;
    clearTimeout(timer);
    try {
      resolve(JSON.parse(line));
    } catch (e) {
      reject(e);
    }
  });
  p.on("exit", (code) => {
    if (pending) {
      clearTimeout(pending.timer);
      pending.reject(Error("Driver exited " + code + " " + stderr));
      pending = null;
    }
  });
  return {
    p,
    ask: (v) =>
      new Promise((resolve, reject) => {
        assert(!pending);
        const timer = setTimeout(() => {
          pending = null;
          p.kill();
          reject(Error("Driver timeout"));
        }, 20000);
        pending = { resolve, reject, timer };
        p.stdin.write(JSON.stringify(v) + "\n");
      }),
  };
}
(async () => {
  server = http.createServer((req, res) => {
    res.setHeader("Content-Type", "text/html");
    res.end(fs.readFileSync(path.join(__dirname, "loop.html")));
  });
  await new Promise((r) => server.listen(0, "127.0.0.1", r));
  const url = "http://127.0.0.1:" + server.address().port;
  chrome = spawn(
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    [
      "--no-first-run",
      "--no-default-browser-check",
      "--disable-extensions",
      "--disable-background-networking",
      "--user-data-dir=" + path.join(tmp, "profile"),
      "--remote-debugging-port=9453",
      "--window-size=1200,800",
      "--window-position=360,140",
      "--app=" + url,
    ],
    { stdio: "ignore" },
  );
  cdp = await connect(9453, (t) => t.url.startsWith(url));
  const { windowId } = await cdp.send("Browser.getWindowForTarget");
  await cdp.send("Browser.setWindowBounds", {
    windowId,
    bounds: { left: 360, top: 140, width: 1200, height: 800 },
  });
  await cdp.send("Page.bringToFront");
  const guard = (activate = false) =>
    sys(path.join(__dirname, "target/release/owned-window"), [
      String(chrome.pid),
      "360",
      "140",
      "1200",
      "800",
      ...(activate ? ["--activate"] : []),
    ]);
  guard(true);
  await sleep(300);
  const geometry = await cdp.ev(
    "({x:screenX,y:screenY+outerHeight-innerHeight,innerWidth,innerHeight,outerWidth,outerHeight,dpr:devicePixelRatio,active:document.hasFocus()})",
  );
  assert.equal(geometry.outerWidth, 1200);
  assert.equal(geometry.outerHeight, 800);
  assert(geometry.active);
  const mx = geometry.x + 88,
    my = geometry.y + 88;
  const cases = [
    {
      id: "pixel",
      op: "capture",
      rect: [mx, my, 1, 1],
      sample: [0, 0],
      delay: 0,
    },
    {
      id: "region",
      op: "capture",
      rect: [mx - 50, my - 50, 100, 100],
      sample: [50, 50],
      delay: 0,
    },
    {
      id: "window",
      op: "capture",
      rect: [360, 140, 1200, 800],
      sample: [mx - 360, my - 140],
      delay: 0,
    },
    {
      id: "loop-zero",
      op: "loop",
      rect: [360, 140, 1200, 800],
      sample: [mx - 360, my - 140],
      delay: 0,
    },
    {
      id: "loop-default",
      op: "loop",
      rect: [360, 140, 1200, 800],
      sample: [mx - 360, my - 140],
      delay: 10,
    },
  ];
  for (const c of cases)
    c.code_points =
      c.id === "pixel"
        ? []
        : Array.from({ length: 16 }, (_, i) => [
            geometry.x + 49 + (i % 4) * 10 - c.rect[0],
            geometry.y + 49 + Math.floor(i / 4) * 10 - c.rect[1],
          ]);
  const source = [
    "loop.html",
    "loop-compare.cjs",
    "loop-robotjs.cjs",
    "src/bin/loop.rs",
    "owned-window.swift",
    "app-cdp.cjs",
    "target/release/loop",
    "target/release/owned-window",
  ];
  const manifest = {
    schema: 1,
    date: new Date().toISOString(),
    pilot: !!pilot,
    cpu: sys("sysctl", ["-n", "machdep.cpu.brand_string"]),
    os: sys("sw_vers", ["-productVersion"]),
    node: process.version,
    rustc: sys("rustc", ["--version"]),
    browser: await cdp.send("Browser.getVersion"),
    source_commit: sys("git", ["-C", root, "rev-parse", "HEAD"]),
    robotjs_commit: sys("git", ["-C", robot, "rev-parse", "HEAD"]),
    sources: Object.fromEntries(
      source.map((p) => [p, sha(path.join(__dirname, p))]),
    ),
    robotjs_binary_sha256: sha(path.join(robot, "build/Release/robotjs.node")),
    geometry,
    cases,
    method:
      "Owned local Chrome review queue. Per trial reset, two animation frames, 50 ms settling and focus/ownership checks outside timer. Alternating grayscale states and a unique 16-bit black/white trial code verify frame freshness with exact colors. The 1x1 case can verify only the alternating state; larger captures verify the code too. Exact magenta target search. Capture timer is capture call only; full-loop timer includes first capture, freshness assertion, exact public findColor, coordinate mapping, native move/click, fresh captures until visible after-token. Both mouse delays explicitly matched (0 or 10 ms per call). No polling sleep. Visible-response deadline 1 second; incomplete loops retain timing but are excluded from successful-latency medians. Five failed preflight calls mark a backend/case unavailable rather than retrying hundreds of known failures. DOM receipts independently verify exactly one trusted click inside target, no focus loss. No retries or recording. Fresh process per backend/round/case, five excluded warmups; six alternating rounds of 20 measured trials. Capture/search/input/render latency and any failed attempts retained. Launch/reset/IPC/DOM receipts excluded from driver timer.",
    runs: [],
  };
  const save = () =>
    fs.writeFileSync(
      path.join(out, "manifest.json"),
      JSON.stringify(manifest, null, 2) + "\n",
    );
  save();
  for (const c of cases) {
    const rounds = pilot ? 1 : 6,
      count = pilot === "rust-only" ? 12 : pilot ? 3 : 20,
      warmups = pilot ? 1 : 5;
    const unavailable = new Set();
    for (let round = 1; round <= rounds; round++)
      for (const language of round % 2
        ? ["robotjs", "rust"]
        : ["rust", "robotjs"]) {
        if (
          unavailable.has(language) ||
          (pilot === "rust-only" && language === "robotjs")
        )
          continue;
        driver = startDriver(language);
        const probe = await driver.ask({ op: "probe" });
        assert(probe.ok);
        assert.equal(probe.result.accessibility, true);
        assert.equal(probe.result.capture, true);
        const run = {
          case: c.id,
          round,
          language,
          probe: probe.result,
          trials: [],
        };
        manifest.runs.push(run);
        for (let n = -warmups; n < count; n++) {
          const id = (round - 1) * count + Math.max(0, n),
            code = ++nonce,
            before = code % 2 ? "000000" : "808080",
            after = "ffffff";
          const state = await cdp.ev(
            `window.reset(${id},${JSON.stringify(before)},${JSON.stringify(after)},${code})`,
          );
          await sleep(50);
          guard();
          assert(state.active);
          const request = { ...c, before, after, code };
          const result = await driver.ask(request);
          const receipt = await cdp.ev("window.state()");
          assert(
            receipt.active && !receipt.focusLost,
            "Focus lost; stop native input",
          );
          guard();
          let verified = result.ok;
          if (result.ok && c.op === "loop") {
            const click = receipt.clicks[0],
              b = receipt.button;
            verified =
              result.result.loop.complete &&
              receipt.complete &&
              receipt.clicks.length === 1 &&
              click.trusted &&
              click.x >= b.x &&
              click.x < b.x + b.width &&
              click.y >= b.y &&
              click.y < b.y + b.height;
          }
          if (c.op === "capture") assert.equal(receipt.clicks.length, 0);
          run.trials.push({
            id,
            warmup: n < 0,
            request,
            ...result,
            receipt,
            verified,
          });
          save();
          if (!pilot && n === -1 && run.trials.every((t) => !t.ok)) {
            run.unavailable =
              "All five preflight calls failed; remaining trials skipped for this backend/case";
            unavailable.add(language);
            save();
            break;
          }
        }
        console.log(
          c.id,
          round,
          language,
          "verified",
          run.trials.filter((t) => !t.warmup && t.verified).length +
            "/" +
            count,
          "failed",
          run.trials
            .filter((t) => !t.verified)
            .map((t) => t.error || "receipt mismatch"),
        );
        driver.p.stdin.end();
        driver = null;
      }
  }
  const screenshot = await cdp.send("Page.captureScreenshot", {
    format: "png",
  });
  fs.writeFileSync(
    path.join(out, "app.png"),
    Buffer.from(screenshot.data, "base64"),
  );
  manifest.app_sha256 = sha(path.join(out, "app.png"));
  save();
  await cdp.send("Browser.close");
  cdp.ws.close();
  server.close();
})().catch(async (e) => {
  console.error(e);
  if (driver) driver.p.kill();
  if (cdp) cdp.ws.close();
  if (chrome) chrome.kill();
  if (server) server.close();
  process.exitCode = 1;
});
