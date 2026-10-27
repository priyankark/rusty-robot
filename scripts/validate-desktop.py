#!/usr/bin/env python3
"""Run owned desktop fixtures serially and retain logs and a machine-readable report."""
import argparse
import datetime
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--secondary", action="store_true", help="Create macOS virtual display and run signed-origin cases")
parser.add_argument("--output", default="target/desktop-validation", help="Artifact directory")
parser.add_argument("--dry-run", action="store_true", help="Print planned commands without sending input")
args = parser.parse_args()
root = Path(__file__).resolve().parent.parent
output = Path(args.output).resolve()
if args.secondary and sys.platform != "darwin":
    parser.error("--secondary currently requires the macOS virtual-display helper")

virtual = ["cargo", "run", "--locked", "--example", "virtual_display", "--"]
desktop = ["cargo", "test", "--locked", "--test", "desktop", "--", "--ignored", "--test-threads=1", "--nocapture"]
omissions = []
if args.secondary:
    desktop = virtual + desktop
else:
    desktop += ["--skip", "captures_secondary_display_at_signed_origin"]
    omissions.append("Secondary-display capture and negative-origin visual workflows were not run")
visual = ["cargo", "test", "--locked", "--test", "visual_desktop", "--", "--ignored", "--nocapture"]
checks = [("inspect", ["cargo", "run", "--locked", "--example", "inspect"], {}),
          ("desktop", desktop, {}), ("primary", visual, {})]
if args.secondary:
    checks.append(("secondary", virtual + visual, {"RUSTY_ROBOT_QA_SECONDARY": "1"}))
if sys.platform == "darwin":
    checks.append(("appkit", ["cargo", "test", "--locked", "--test", "native_macos", "--", "--ignored", "--nocapture"], {}))
report = {"started": datetime.datetime.now(datetime.timezone.utc).isoformat(),
          "platform": platform.platform(), "omissions": omissions, "results": []}
if args.dry_run:
    print(json.dumps({"checks": checks, "omissions": omissions}, indent=2))
    sys.exit(0)

output.mkdir(parents=True, exist_ok=True)
exit_code = 0
for name, command, extra in checks:
    env = os.environ.copy()
    env.pop("RUSTY_ROBOT_QA_SECONDARY", None)
    env.update(extra)
    env["RUSTY_ROBOT_QA_ARTIFACTS"] = str(output / name)
    log = output / (name + ".log")
    print(f"Running {name}; log: {log}", flush=True)
    start = time.monotonic()
    with log.open("w", encoding="utf-8") as stream:
        result = subprocess.run(command, cwd=root, env=env, stdout=stream, stderr=subprocess.STDOUT)
    content = log.read_text(encoding="utf-8", errors="replace")
    counts = re.findall(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", content)
    test_counts = [dict(zip(["passed", "failed", "ignored"], map(int, row))) for row in counts]
    effective_code = result.returncode
    validation_error = None
    if name != "inspect" and effective_code == 0 and (
        not test_counts or sum(row["passed"] for row in test_counts) == 0
        or any(row["failed"] or row["ignored"] for row in test_counts)
    ):
        effective_code = 1
        validation_error = "Expected executed tests with no failures or ignored cases"
    report["results"].append({"name": name, "command": command, "exit_code": effective_code,
                              "seconds": round(time.monotonic()-start, 2),
                              "test_counts": test_counts, "validation_error": validation_error})
    report["complete"] = len(report["results"]) == len(checks)
    report["passed"] = report["complete"] and all(r["exit_code"] == 0 for r in report["results"])
    (output / "report.json").write_text(json.dumps(report, indent=2)+"\n", encoding="utf-8")
    print("\n".join(content.splitlines()[-12:]), flush=True)
    if effective_code:
        if validation_error:
            print(validation_error, flush=True)
        exit_code = effective_code
        break
sys.exit(exit_code)
