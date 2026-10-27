#!/usr/bin/env python3
"""Run consumer contracts against crates.io, never a workspace/path dependency."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--version', default='0.1.2')
parser.add_argument('--output', default='target/registry-dogfood')
parser.add_argument('--desktop', action='store_true', help='Also control owned Chromium/AppKit windows on an unlocked desktop')
args = parser.parse_args()
if not re.fullmatch(r'\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?', args.version):
    parser.error('Expected a semantic package version')
root = Path(__file__).resolve().parent.parent
output = Path(args.output).resolve()
output.mkdir(parents=True, exist_ok=True)
report = {'package': 'rusty-robot-desktop', 'version': args.version, 'checks': [], 'desktop': args.desktop}

with tempfile.TemporaryDirectory(prefix='rusty-registry-dogfood-') as directory:
    consumer = Path(directory)
    (consumer / 'src').mkdir()
    (consumer / 'src/main.rs').write_text('fn main() { let _robot = rusty_robot::Robot; }\n')
    (consumer / 'Cargo.toml').write_text(f'''[package]
name = "registry-dogfood"
version = "0.0.0"
edition = "2024"
[features]
default = ["png"]
png = ["rusty-robot-desktop/png"]
[dependencies]
rusty-robot-desktop = {{ version = "={args.version}", default-features = false }}
[dev-dependencies]
libc = "0.2"
crc32fast = "1"
flate2 = "1"
serde_json = "1"
tempfile = "3"
png = "0.18"
[target.'cfg(target_os = "macos")'.dev-dependencies]
objc2 = "0.6"
objc2-foundation = "0.3"
''')
    # Reuse public API contracts as an external application. No library source is copied.
    shutil.copytree(root / 'tests', consumer / 'tests', ignore=shutil.ignore_patterns('node_modules', 'package-lock.json'))
    env = os.environ.copy()
    for key in ['CARGO_TARGET_DIR', 'RUSTY_ROBOT_QA_SECONDARY', 'RUSTY_ROBOT_QA_ARTIFACTS']:
        env.pop(key, None)
    # Fixtures embed CARGO_MANIFEST_DIR: never reuse a binary compiled for a
    # previous TemporaryDirectory, even when copied source mtimes are identical.
    env['CARGO_TARGET_DIR'] = str(output / 'build' / consumer.name)
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--format-version=1'], cwd=consumer, env=env))
    package = next(p for p in metadata['packages'] if p['name'] == 'rusty-robot-desktop')
    assert package['source'] == 'registry+https://github.com/rust-lang/crates.io-index', package['source']
    assert package['version'] == args.version
    report['source'] = package['source']
    commands = [('default', ['cargo', 'test', '--locked']),
                ('bmp-only', ['cargo', 'test', '--locked', '--no-default-features'])]
    if args.desktop:
        # The harness needs Electron; the library still resolves exclusively from crates.io.
        os.symlink(root / 'tests/gui/node_modules', consumer / 'tests/gui/node_modules', target_is_directory=True)
        commands.append(('chromium', ['cargo', 'test', '--locked', '--test', 'visual_desktop', '--', '--ignored', '--nocapture']))
        if os.sys.platform == 'darwin':
            commands.append(('appkit', ['cargo', 'test', '--locked', '--test', 'native_macos', '--', '--ignored', '--nocapture']))
    for name, command in commands:
        log = output / (name + '.log')
        env['RUSTY_ROBOT_QA_ARTIFACTS'] = str(output / name)
        with log.open('w') as stream:
            result = subprocess.run(command, cwd=consumer, env=env, stdout=stream, stderr=subprocess.STDOUT, timeout=600)
        counts = re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored', log.read_text())
        totals = {key: sum(int(row[i]) for row in counts) for i, key in enumerate(['passed', 'failed', 'ignored'])}
        passed = result.returncode == 0 and totals['passed'] > 0 and totals['failed'] == 0
        if name in ['chromium', 'appkit']:
            passed = passed and totals['ignored'] == 0
        report['checks'].append({'name': name, 'success': passed, **totals})
        report['complete'] = len(report['checks']) == len(commands)
        report['passed'] = report['complete'] and all(check['success'] for check in report['checks'])
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(f'{name}: {totals}; passed={passed}', flush=True)
        if not passed:
            raise SystemExit(f'Failed: {log}')
print('External crates.io consumer verified; no workspace dependency used.')
