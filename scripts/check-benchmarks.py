#!/usr/bin/env python3
"""Validate the public benchmark evidence independently of HTML rendering."""
import csv
import json
import math
import statistics
from pathlib import Path

root = Path(__file__).resolve().parents[1]
source = root / "benchmarks/2026-10-27"
raw = json.loads((source / "raw.json").read_text())
assert not raw["errors"], "Recorded run has errors"
rows = list(csv.DictReader((source / "summary.csv").open()))
assert len(rows) == 21 and len({r["case"] for r in rows}) == 21
assert len(raw["results"]) == 126
for row in rows:
    for language in ("rust", "robotjs"):
        records = [r for r in raw["results"] if r["case"] == row["case"] and r["language"] == language]
        assert sorted(r["round"] for r in records) == [1, 2, 3]
        assert all(len(r["ns_per_op"]) == 7 for r in records)
        samples = [v for r in records for v in r["ns_per_op"]]
        assert all(math.isfinite(v) and v > 0 for v in samples)
        assert math.isclose(statistics.median(samples) / 1000, float(row[language + "_us"]), rel_tol=1e-12)
        round_values = [statistics.median(r["ns_per_op"]) / 1000 for r in records]
        for label, fn in (("min", min), ("max", max)):
            assert math.isclose(fn(round_values), float(row[language + "_" + label + "_round_us"]), rel_tol=1e-12)
    assert math.isclose(float(row["robotjs_over_rust"]), float(row["robotjs_us"]) / float(row["rust_us"]), rel_tol=1e-12)
    if row["case"] not in ("get_mouse_pos", "get_pixel_color"):
        assert len({r["observed"] for r in raw["results"] if r["case"] == row["case"]}) == 1
ab = json.loads((source / "optimization-ab.json").read_text())
assert len(ab["results"]) == 78
cases = {r["case"] for r in ab["results"]}
assert len(cases) == 13
for case in cases:
    for version in ("before", "after"):
        records = [r for r in ab["results"] if r["case"] == case and r["version"] == version]
        assert sorted(r["round"] for r in records) == [1, 2, 3]
        assert all(len(r["ns_per_op"]) == 7 and all(math.isfinite(n) and n > 0 for n in r["ns_per_op"]) for r in records)
print("Verified 21 Rust/RobotJS comparisons, 13 A/B comparisons, medians, ranges, and 1,428 timed batches.")
