#!/usr/bin/env python3
"""Stage the benchmark report, raw evidence, and generated API docs for Pages."""
from pathlib import Path
import shutil
root = Path(__file__).resolve().parents[1]
out = root / "target/site"
if out.exists():
    shutil.rmtree(out)
out.mkdir(parents=True)
html = (root / "docs/benchmark-report.html").read_text()
html = html.replace('../assets/robot.png', 'assets/robot.png')
for name in ('optimization-results.md', 'benchmarks.md'):
    html = html.replace('href="' + name + '"', 'href="https://github.com/priyankark/rusty-robot/blob/main/docs/' + name + '"')
(out / 'index.html').write_text(html)
(out / '.nojekyll').touch()
shutil.copytree(root / 'assets', out / 'assets')
shutil.copytree(root / 'benchmarks', out / 'benchmarks')
shutil.copytree(root / 'target/doc', out / 'api')
print(out)
