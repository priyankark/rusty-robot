#!/usr/bin/env python3
"""Ensure evidence validation rejects corrupted receipts and hidden failures."""
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = Path('docs/workflows/2026-10-31-loop')


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.manifest = json.loads((ROOT/EVIDENCE/'manifest.json').read_text())
        files = ['scripts/loop-report.py', 'benchmark.md',
                 'docs/building-rusty-robot.html',
                 'tools/benchmark-demo/capture-diagnostic.c']
        files += [str(EVIDENCE/name) for name in
                  ('manifest.json', 'summary.json', 'diagnostics.json', 'app.png')]
        files += ['tools/benchmark-demo/'+name for name in self.manifest['sources']
                  if not name.startswith('target/')]
        for name in set(files):
            dest = self.root/name
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT/name, dest)

    def validate(self, valid):
        (self.root/EVIDENCE/'manifest.json').write_text(json.dumps(self.manifest))
        result = subprocess.run([sys.executable, str(self.root/'scripts/loop-report.py'), '--check'],
                                text=True, capture_output=True)
        if valid:
            self.assertEqual(result.returncode, 0, result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('AssertionError', result.stderr)

    def run_for(self, case):
        return next(r for r in self.manifest['runs']
                    if r['language'] == 'rust' and r['case'] == case)

    def test_original_evidence_passes(self):
        self.validate(True)

    def test_wrong_click_coordinate_is_rejected(self):
        trial = next(t for t in self.run_for('loop-default')['trials'] if not t['warmup'])
        trial['receipt']['clicks'][0]['x'] += 10
        self.validate(False)

    def test_stale_frame_id_is_rejected(self):
        trial = next(t for t in self.run_for('region')['trials'] if not t['warmup'])
        trial['result']['first_code'] ^= 1
        self.validate(False)

    def test_dropped_failure_is_rejected(self):
        trials = self.run_for('loop-zero')['trials']
        failed = next(t for t in trials if not t['warmup'] and not t['verified'])
        trials.remove(failed)
        self.validate(False)


if __name__ == '__main__':
    unittest.main()
