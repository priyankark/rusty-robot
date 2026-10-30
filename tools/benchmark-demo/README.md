# Side-by-side API demonstrations

The Node runner calls the real RobotJS native addon. The Rust runner calls this
repository’s Rust library. Both read the same fixtures, warm up five times,
wait for `go`, then run fixed equal operation counts. Every answer is checked.
They run concurrently; checks and progress IPC are included in elapsed time.
These videos are demonstrations, not replacements for isolated benchmarks.

The recorder screen-grabs an owned Chrome page receiving live progress. It
encodes frames using their wall-clock timestamps, with a short opening and end
hold. No personal desktop contents are captured. Videos are silent; readable
results and descriptions accompany them on the page.

Requirements: Node 24+, Rust, Chrome at the macOS application path, ffmpeg.
Build the pinned RobotJS reference and fixtures with `scripts/benchmark.py`
(see `docs/benchmarks.md`). Then:

```sh
cargo build --release --locked --manifest-path tools/benchmark-demo/Cargo.toml
node tools/benchmark-demo/record.cjs /path/to/robotjs /path/to/fixtures /new/evidence/directory
```

Use a new evidence directory for each recording. Never overwrite published runs.
The gallery uses `docs/demos/2026-10-29`. Regenerate it with
`python3 scripts/demo-report.py`; CI checks the gallery, result agreement, and
video SHA-256 hashes with `--check`.

Cases: 1,000 dense color-result collections; 5,000 absent-color scans;
1,500 late template matches; 5,000 BMP loads. BMP loads check a known pixel as
well as dimensions. Output formats differ (RobotJS BGR24, Rust BGRA32).

## Actual desktop workflows

`workflow.html` is an owned interactive app. The `desktop` Rust binary and
`desktop-robotjs.cjs` send native OS input; the task-board workflow also uses
each library's real screen capture and color search. The harness uses DOM reads
only for geometry of the owned window and field/result verification. Target
positions are found from captured pixels. It stops on loss of focus.

On an unlocked Mac with Accessibility and Screen Recording permission:

```sh
cargo build --release --locked --manifest-path tools/benchmark-demo/Cargo.toml --bin desktop
node tools/benchmark-demo/workflow-record.cjs /path/to/robotjs /new/output/directory
```

Keep the owned app focused during the run. Each implementation runs sequentially;
recordings are aligned at task start, played at 1x speed, and combined side by side.
Both retain their default 10 ms input delays. UI verification, IPC, display
presentation, and recording overhead are included. The completed pane freezes
while the other finishes. No synthetic input is dispatched to fill the form.

Published run: `docs/workflows/2026-10-30`. Eight complete customer records were
verified for both implementations. Rust completed 48 screenshot/search/click
cycles with no misses. RobotJS capture returned an error on this host; this is
retained as a failed run, not converted into a speedup. `workflow-report.py`
checks the outcomes and video hashes and builds the archived default-behavior gallery.


## Controlled comparison (primary gallery)

The default-delay form demo is not a matched typing-rate benchmark: RobotJS
waits between characters, while Rust batches text. Use `workflow-compare.cjs`
for the corrected comparison:

```sh
cargo build --release --locked --manifest-path tools/benchmark-demo/Cargo.toml --bin desktop
node tools/benchmark-demo/workflow-compare.cjs /path/to/robotjs /new/output/directory
python3 scripts/controlled-workflow-report.py --check
```

Both profiles set mouse delay to 10 ms and key-tap delay to 0 ms. The first
uses each API's ordinary text entry with keyboard delay 0; Rust retains its
internal 10 ms Unicode-batch safety interval. The second explicitly requests
6,000 CPM from both paced-typing APIs. The harness waits for the expected field
to receive focus before typing and verifies every field before advancing.

Each profile runs one complete warmup per library, then six trials per library
in alternating order, without video recording. A separate pair of illustrative
runs is recorded afterward; those timings are not the measured medians.
Published evidence is `docs/workflows/2026-10-30-controlled`. All 24 measured
runs passed. No general language-speed claim follows from this one app and Mac.

A preliminary zero-mouse-delay pilot stopped on lost focus. Measured profiles
use the same 10 ms mouse delay and explicit field-focus checks for both libraries.
The earlier RobotJS capture error is excluded from the speed comparison.


## Screenshot-analysis workload (featured demo)

The featured comparison detects known green Ready badges across 60 distinct
1920×1080 generated dashboard screenshots, then groups matching pixels into
rectangles. Both implementations use the public find-all-color API and the same
row-major grouping steps. Exact results are checked against badge rectangles
recorded independently from the fixture DOM. This is a known-color UI workload,
not OCR or arbitrary screenshot understanding.

The input archive is `docs/workflows/2026-10-30-dashboard/screenshots.zip`.
Extract it into an empty directory. Decode each PNG to BGRA with ffmpeg, and
verify the output SHA-256 against `fixtures.json` before running:

```sh
ffmpeg -i 00.png -f rawvideo -pix_fmt bgra 00.bgra
# Repeat for 00.png through 59.png; hashes are in fixtures.json.
cargo build --release --locked --manifest-path tools/benchmark-demo/Cargo.toml --bin dashboard
node tools/benchmark-demo/dashboard-record.cjs /path/to/robotjs /path/to/fixtures /new/output/directory
python3 scripts/dashboard-report.py --check
```

`make-dashboard-fixtures.cjs OUTPUT` regenerates the corpus with Chrome. Use
the archived PNGs for byte-identical reproduction. File reads, decoding and a
complete warmup are outside timing. Six rounds alternate library order; video
recording is disabled. Additional sequential recorded takes illustrate the
results; progress IPC and recording make those timings different. All measured
outputs are retained. The result measures public API allocation plus grouping,
not every possible optimized JavaScript buffer-scanning implementation.

The primary gallery leads with this measured Rust win and links the controlled
typing comparison, where zero-keyboard-delay Node was faster and equal-rate
text entry was tied. Earlier default-delay videos remain explicitly qualified.


## More screenshot workloads: alerts and test heatmaps

The main write-up embeds all three screenshot comparisons and both controlled
native typing videos. `scripts/workload-report.py --check` verifies all 36
measured screenshot batches, six video event streams, alternating order, exact
expected answers, and corpus/video hashes before checking generated article text.
It also checks the heatmap's raw pixel counts, before any division into test tiles.

- `alerts`: 60 monitoring screenshots, 40 services each. Twelve have one failed
  badge; 48 have none. Both public first-color APIs locate the exact red pixel,
  then construct a known-size 100×28 rectangle. Expected geometry comes from
  the fixture DOM. Search and result construction are timed.
- `heatmap`: 60 screenshots with 10,800 tiles each. Both public color-count APIs
  count exact green pixels. Each passing tile covers 70 pixels. Raw counts must
  equal independently generated expectations ×70; conversion to test counts
  happens after timing. These fixtures contain 547,560 passing checks.

These cases return small results, unlike the original find-all-color workload.
They use known synthetic layouts, not arbitrary UI recognition. All three use
identical preloaded BGRA32 inputs, a complete verified warmup, six alternating
trials per library, and no video recording during measurement. Each illustrative
video is a separate pair of sequential runs; its timings include progress and
recording overhead. Loading, decoding, and screen capture are outside the timer.

Use the archived `screenshots.zip` in
`docs/workflows/2026-10-30-alerts/` or `docs/workflows/2026-10-30-heatmap/` for exact
inputs. Extract into an empty directory, decode every PNG with ffmpeg as in the
previous section, and retain `fixtures.json`. The recorder verifies all raw BGRA
hashes before starting. Chrome, ffmpeg, macOS, and the pinned RobotJS addon are
required by this recording harness.

```sh
cargo build --release --locked --manifest-path tools/benchmark-demo/Cargo.toml --bin workloads
node tools/benchmark-demo/workload-record.cjs alerts /path/to/robotjs /path/to/alerts-fixtures /new/alerts-results
node tools/benchmark-demo/workload-record.cjs heatmap /path/to/robotjs /path/to/heatmap-fixtures /new/heatmap-results
python3 scripts/workload-report.py --check
```

To generate new inputs instead, use `node tools/benchmark-demo/make-workload-fixtures.cjs
/new/fixtures alerts` (or `heatmap`). Regenerated rendering may differ from the
archived PNGs. Keep new measurements in new directories; never overwrite a
published experiment. The article generator targets the checked-in evidence,
not an arbitrary new run. The manifests retain source commits, binary/source
hashes, environment, and each trial time. No library release is needed for
these benchmark additions.
