# Rust-driven Chrome dogfood — 2026-11-04

Rusty Robot successfully drove real Chrome through Wikipedia and Rust Playground.
The useful next layer is session and observation support around the existing
input primitives. This run did not identify an input-delivery failure.

## What ran

- Library source: `ebb5b9d`, package 0.1.2, release build; macOS 27.0 arm64,
  Chrome 155.0.8059.40, one 1920×1080 display at 1× scale.
- A separate Chrome process with a temporary profile, removed after the session.
- All mouse, keyboard, and screenshots went through the **Rust API**. No Node
  bindings, Playwright, CDP, DOM selectors, or injected browser events drove it.
- The agent selected coordinates by viewing PNG screenshots. The driver used
  `Bitmap::to_screen_point` to convert image coordinates to desktop coordinates.
- Outcomes were checked visually in the real applications; these were not
  machine assertions about DOM state or exact text equality.

24 native input commands completed. Two additional commands were refused by the
driver because Chrome was not active. The terminal hosting the agent became
foreground between steps; explicitly activating the owned Chrome PID recovered
the session. This is an integration finding, not evidence of lost native input.

| Workflow | Observed outcome | Local screenshot |
| --- | --- | --- |
| Open Wikipedia, enter a search, submit | Rust article loaded | `frame-009.png` |
| Follow Memory safety, then use browser Back | Both article transitions succeeded | `frame-012.png`, `frame-013.png` |
| Scroll the article | −6 moved it about 6 pixels; −600 about 600 pixels | `frame-014.png`, `frame-015.png` |
| Command-F and type `ownership` | Find displayed 1/9 and highlighted a match | `frame-017.png` |
| Drag-select text; double-click a word | Selection visible; double-click selected “Rust” | `frame-019.png`, `frame-020.png` |
| Open a new tab, visit rust-lang.org, follow Playground | Both sites loaded | `frame-024.png`, `frame-026.png` |
| Replace editor contents with multiline Unicode Rust; Run | Compilation succeeded; output contained `Rusty Robot: café 中文 🚀` | `frame-030.png`, `frame-032.png` |
| Drag Playground's split-pane divider | Divider moved from approximately x=950 to x=1195 | `frame-033.png` |

The original local evidence is in `target/cua-chrome/trace.jsonl` and adjacent
screenshots. Frame 11 includes the foreground terminal; frame 12 records the same
article after Chrome was focused again. Screenshots are local artifacts, not
included in a published package or committed report.

## Improvements, in order

1. **Target and focus support.** Add an optional native window/process layer that
   can identify, activate, and check a target. This driver needed AppKit FFI
   outside Rusty Robot. Check immediately before input, expose focus loss clearly,
   and leave activation policy with the consumer. A focus check alone cannot
   eliminate the race where focus changes during a sequence.
2. **Observation helpers with deadlines.** Offer capture polling with a predicate,
   bounded timeouts, and useful last-frame diagnostics. A fixed 150 ms delay
   captured Playground's progress indicator (frame 31), while a later capture
   showed completed output (frame 32). Visual stability is a useful hint, not a
   guarantee of network or application completion.
3. **A higher-level action/session layer.** A drag currently requires press,
   interpolated movement, release, and error cleanup in every consumer. Provide
   a scoped held-button guard and a `drag_to` helper, plus ordered sequences and
   cancellation. Frame-associated positions would also reduce stale-coordinate
   mistakes; a frame ID does not by itself prove the UI has stayed unchanged.
4. **Optimize the screenshot path for agent consumption.** Measure capture,
   encoding, and transport separately. Expose an ergonomic in-memory PNG/JPEG
   route with resize and coordinate metadata, reusing the existing Rust media
   crate where practical. The viewer rejected BMP, so this run used PNG files.
   Prefer configurable fast encoding and region capture before optimizing
   already short click/key calls.
5. **Explicit text and scroll semantics.** Distinguish typing from literal paste
   in documentation and any future API. Playground auto-indented the closing
   brace; the program ran, but typing did not preserve exact source formatting.
   A paste mode should preserve clipboard state and remain opt-in. Expose scroll
   units explicitly: this Mac/Chrome run behaved like pixel scrolling; that does
   not establish equivalent units on other backends.

## Measurements

For 32 logged full-display frames during interactive browsing (excluding the
initial capture), median capture was **49.23 ms**, range 35.09–64.45 ms. Median
PNG encoding plus file saving was **72.95 ms**, range 58.17–94.97 ms. These exclude
the intentional 150 ms settle delay, tool transport, image viewing, and model
reasoning. Encoding time was not isolated from file writing.

A separate sequential burst of 12 samples per case measured:

| Capture request | Median | Range |
| --- | ---: | ---: |
| One pixel | 28.00 ms | 26.60–38.97 ms |
| Main display, explicit rectangle | 30.77 ms | 30.20–32.71 ms |
| Main display, default selection | 29.88 ms | 29.04–30.78 ms |

The original trace calls the explicit full-display rectangle `window`; it is
**not** a window-only capture. The example now labels it `display_rect`.
The near-full-display cost of one-pixel capture agrees with the backend's current
full-display capture followed by cropping. These are small single-host samples,
not cross-platform benchmarks or a before/after optimization comparison.

## Reuse the driver

```sh
cargo run --release --locked --example cua
```

Requires macOS, Chrome at its standard application path, an unlocked interactive
session, Accessibility, and Screen Recording access. The example opens its own
Chrome profile. It captures the main display, so other visible windows may appear
in evidence. Output defaults to a new `target/cua-chrome/<timestamp>` directory;
`RUSTY_ROBOT_CUA_OUTPUT` selects an unused output directory.

Send one JSON object per line. Read the screenshot and response between actions:

```json
{"action":"focus"}
{"action":"capture"}
{"action":"click","frame":2,"x":900,"y":605}
{"action":"type","text":"Rust (programming language)"}
{"action":"key","key":"enter"}
{"action":"benchmark"}
{"action":"quit"}
```

Coordinates above are illustrative, not a replay script. Use the actual latest
frame ID and visible target. Other commands are `observe` (PID/active state),
`scroll` (`frame`, `x`, `y`, `dx`, `dy`) and `drag` (`frame`, `x`, `y`, `toX`,
`toY`); `click` supports `button` and `double`; `key` supports `modifiers`.
Each successful input action captures another frame. Explicit `focus` is needed
if the terminal takes focus again. Use `quit` or EOF for normal cleanup; forced
termination is not a verified cancellation path.

This example is a deliberately small synchronous consumer, not a production
automation service. Windows/Linux, mixed-DPI monitors, long typing workloads,
focus changes mid-action, and interrupted drags were not tested in this run.

Validation after adding the example: `cargo fmt --check` and Clippy with
`-D warnings` passed for both feature configurations. `cargo test --all-features
--locked` passed 118 tests; `cargo test --no-default-features --locked` passed
108. Each run left 36 opt-in desktop tests ignored; the interactive Chrome
workflows above were executed separately. No production library code was changed.

## Follow-up dogfood and GitHub issues

A second disposable Chrome session completed **21 more native input commands**,
for 45 across the two sessions. Evidence is in
`target/cua-chrome/1793818763926/trace.jsonl` and its adjacent screenshots.
This session also verified right-click context menus, middle-click opening a
background tab, Command-3 tab selection, 125% browser zoom, and clicking a link
using newly observed coordinates after page reflow. A deliberately stale frame
ID was rejected by the consumer before input. This tests the consumer guard,
not a built-in Rusty Robot stale-frame guarantee.

The multiline-text finding now has a semantic reproduction. Typing this source
into Rust Playground with `Robot.type_string`, then clicking Run, compiled but
failed its assertion:

```rust
fn main() {
    let s = "alpha
beta";
    assert_eq!(s, "alpha\nbeta");
    println!("literal text preserved");
}
```

The editor added four spaces before `beta`; execution reported left
`"alpha\n    beta"` versus right `"alpha\nbeta"` and exit status 101
(`frame-015.png`). A control with the same statements but no leading indentation
passed (`frame-010.png`). This is an editor interaction requiring an explicit
literal insertion mode, not evidence of native event loss.

The same captured Wikipedia image was also processed by the existing Rust media
crate. One warmup round was omitted, then four cases rotated over 12 measured
rounds each:

| Existing encoding path | Median ms | Bytes |
| --- | ---: | ---: |
| Core PNG file save, 1920×1080 | 73.22 | 573,135 |
| Media PNG memory buffer, 1920×1080 | 6.67 | 704,851 |
| Media PNG memory buffer, 1440×810 | 5.98 | 582,923 |
| Media JPEG q85 memory buffer, 1440×810 | 10.20 | 230,100 |

Core timing includes file writing; media timing ends at encoded bytes. Image
decoding, validation, and media evidence writes were outside timing. Both
full-resolution PNGs decoded to identical source RGB pixels. All outputs decoded
at the expected dimensions, and the resized JPEG was visually inspected. This
compares available consumer paths with different settings, not isolated encoder
performance. The existing media crate is unpublished and its `Frame` does not
preserve desktop origin/scale, so the issue requests reuse and exposure of that
implementation rather than inventing another encoder.

The measurement consumer is saved under `target/cua-encode`; its JSON samples
are `target/cua-chrome/1793818763926/encode-measurements.json`.
A second capture burst measured median 29.17 ms for one pixel, 32.02 ms for an
explicit display rectangle, and 31.24 ms for default full-display capture
(12 samples each). No Windows/Linux native claims were added.

No existing open or closed issues were found before filing. Created issues:

- [#29: Literal-text insertion](https://github.com/priyankark/rusty-robot/issues/29)
- [#30: Target and focus APIs](https://github.com/priyankark/rusty-robot/issues/30)
- [#31: Deadline-bounded observation](https://github.com/priyankark/rusty-robot/issues/31)
- [#32: Scoped input ownership and drag cleanup](https://github.com/priyankark/rusty-robot/issues/32)
- [#33: Fast Rust screenshot encoding with coordinate metadata](https://github.com/priyankark/rusty-robot/issues/33)
- [#34: macOS region-capture performance](https://github.com/priyankark/rusty-robot/issues/34)
- [#35: Explicit scroll units](https://github.com/priyankark/rusty-robot/issues/35)

Each issue distinguishes live evidence from source inspection or proposed
behavior, and includes environment details and acceptance criteria. No desktop
screenshots or unrelated user-window contents were uploaded. Both owned Chrome
processes were closed after their sessions.
