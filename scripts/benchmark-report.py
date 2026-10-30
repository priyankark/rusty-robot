#!/usr/bin/env python3
"""Render the recorded benchmarks as a standalone, offline HTML report."""
import argparse
import csv
import json
import statistics
from pathlib import Path

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check', action='store_true', help='Fail if the checked-in report differs')
args = parser.parse_args()
source = root / 'benchmarks/2026-10-27'
ab = json.loads((source / 'optimization-ab.json').read_text())
current = json.loads((source / 'raw.json').read_text())
rows = []
for case in dict.fromkeys(r['case'] for r in ab['results']):
    values = {version: statistics.median([
        n for r in ab['results'] if r['case'] == case and r['version'] == version
        for n in r['ns_per_op']
    ]) / 1000 for version in ['before', 'after']}
    rows.append({'case': case, **values})
with (source / 'summary.csv').open() as f:
    comparison = [{'case': r['case'], 'before': float(r['robotjs_us']),
                   'after': float(r['rust_us'])} for r in csv.DictReader(f)]
cases = json.loads((root / 'scripts/benchmark-cases.json').read_text())
assert set(cases) == {r['case'] for r in comparison}
payload = json.dumps({'ab': rows, 'js': comparison, 'cases': cases, 'metadata': current['metadata']}, ensure_ascii=False).replace('<', '\\u003c')
data_script = 'window.RUSTY_BENCHMARKS = ' + payload + ';\n'
template = (root / 'docs/templates/benchmark-report.html').read_text()
rendered = template.replace('__STYLE__', (root / 'docs/site.css').read_text() + (root / 'docs/benchmark-explorer.css').read_text()).replace('__DATA__', data_script).replace('__SCRIPT__', (root / 'docs/benchmark-explorer.js').read_text())
for output, content in [(root / 'docs/benchmark-report.html', rendered), (root / 'docs/benchmark-data.js', data_script)]:
    if args.check:
        if not output.exists() or output.read_text() != content:
            raise SystemExit(f'Report differs: run python3 scripts/benchmark-report.py ({output.name})')
    else:
        output.write_text(content)
    print(output)
