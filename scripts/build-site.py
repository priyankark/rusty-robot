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
(out / 'api-benchmarks.html').write_text(html)
consumer = (root / 'docs/consumer-benchmarks.html').read_text().replace('../assets/robot.png', 'assets/robot.png')
(out / 'consumer-benchmarks.html').write_text(consumer)
for source, destination in [('site-index.html', 'index.html'),
                            ('building-rusty-robot.html', 'building-rusty-robot.html'),
                            ('site.css', 'site.css'),
                            ('benchmark-explorer.css', 'benchmark-explorer.css'),
                            ('benchmark-explorer.js', 'benchmark-explorer.js'),
                            ('workload-videos.js', 'workload-videos.js'),
                            ('benchmark-data.js', 'benchmark-data.js'),
                            ('demos.html', 'demos.html'),
                            ('api-operation-demos.html', 'api-operation-demos.html'),
                            ('default-workflow-demos.html', 'default-workflow-demos.html'),
                            ('typing-comparison.html', 'typing-comparison.html')]:
    shutil.copyfile(root / 'docs' / source, out / destination)
shutil.copytree(root / 'docs/demos', out / 'demos')
shutil.copytree(root / 'docs/workflows', out / 'workflows')
(out / '.nojekyll').touch()
shutil.copytree(root / 'assets', out / 'assets')
shutil.copytree(root / 'benchmarks', out / 'benchmarks')
shutil.copytree(root / 'target/doc', out / 'api')
print(out)
