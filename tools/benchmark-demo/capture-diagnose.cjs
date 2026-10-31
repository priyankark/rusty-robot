// Run separately, after timing: compare the same legacy API in C and Node.
const { execFileSync } = require("node:child_process");
const fs = require("node:fs"),
  path = require("node:path"),
  crypto = require("node:crypto");
const [original, rebuilt, output, nodeHeaders] = process.argv.slice(2);
if (!nodeHeaders)
  throw Error(
    "capture-diagnose.cjs ORIGINAL_ROBOTJS REBUILT_ROBOTJS OUTPUT_JSON NODE_HEADERS",
  );
if (fs.existsSync(output)) throw Error("Use new output file");
const source = path.join(__dirname, "capture-diagnostic.c"),
  temp = fs.mkdtempSync("/tmp/rusty-capture-diagnostic-");
const sha = (p) =>
  crypto.createHash("sha256").update(fs.readFileSync(p)).digest("hex");
const cmd = (p, a) => execFileSync(p, a, { encoding: "utf8", timeout: 60000 });
cmd("clang", [
  source,
  "-framework",
  "ApplicationServices",
  "-o",
  temp + "/capture",
]);
cmd("clang", [
  "-shared",
  "-undefined",
  "dynamic_lookup",
  "-DNODE_DIAGNOSTIC",
  "-I",
  nodeHeaders,
  source,
  "-framework",
  "ApplicationServices",
  "-o",
  temp + "/capture.node",
]);
const result = {
  date: new Date().toISOString(),
  source_sha256: sha(source),
  node: process.version,
  standalone: JSON.parse(cmd(temp + "/capture", [])),
  node_context: JSON.parse(
    cmd(process.execPath, [
      "-e",
      "require(process.argv[1]).capture()",
      temp + "/capture.node",
    ]),
  ),
  robotjs: {},
};
for (const [name, folder] of [
  ["original", original],
  ["rebuilt", rebuilt],
]) {
  const root = path.resolve(folder);
  const probe = JSON.parse(
    cmd(process.execPath, [
      "-e",
      `const r=require(process.argv[1]);const result={permission:r.getScreenCapturePermission()};const start=performance.now();try{const b=r.screen.capture(360,140,100,100);result.returned_image=true;result.width=b.width;result.height=b.height}catch(e){result.returned_image=false;result.error=e.message}result.elapsed_ms=performance.now()-start;console.log(JSON.stringify(result));`,
      root,
    ]),
  );
  result.robotjs[name] = {
    ...probe,
    source_commit: cmd("git", ["-C", root, "rev-parse", "HEAD"]).trim(),
    source_clean:
      cmd("git", ["-C", root, "status", "--porcelain"]).trim() === "",
    binary_sha256: sha(path.join(root, "build/Release/robotjs.node")),
  };
}
fs.writeFileSync(output, JSON.stringify(result, null, 2) + "\n");
console.log(JSON.stringify(result, null, 2));
