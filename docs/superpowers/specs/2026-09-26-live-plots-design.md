# Live plots: updating a shown plot in place

**Spec for 0.4.0** · 2026-09-26 · Status: approved; umbrella issue #21, work in #47–#52

## Goal

Finish issue #21: a plot shown in the terminal can be updated after it is drawn, so live data
looks like a moving chart instead of a stack of plots. The library gets an API for replacing a
shown plot frame by frame, and the CLI gets a follow mode that plots streaming stdin.

**Success criteria**

1. `let mut live = plot.show_live()?;` draws the plot, and every `live.update(&plot)?` after a
   change to the plot's data replaces it in place, without flicker, in Kitty, Ghostty, WezTerm
   and Konsole, and through tmux passthrough.
2. `tail -f data.csv | termplt --follow` shows the same plot growing as lines arrive, with
   `--window N` keeping a sliding window and `--interval MS` bounding the frame rate.
3. Both cases from the issue work: a point inside the current axes changes only the series, and
   a point outside them rescales the axes, ticks and grid. The caller does nothing different.
4. Nothing else changes: `Plot::show`, `save_png` and `render` produce the same bytes as before,
   and every golden image still matches.
5. Every frame the CLI sends is decoded and checked by a pty test, including the cursor
   movements around it and the text printed after the stream ends.
6. No new dependencies; the MSRV stays 1.88.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| How a frame reaches the terminal | A full re-render sent under a fresh image id (`a=t`), placed over the previous frame (`a=p`, `C=1`), then the previous id deleted (`a=d,d=I`), all in one write | Only the base protocol is needed, which every supported terminal implements. Retransmitting under the *same* id makes the terminal delete the old image before the new data has finished arriving, which blinks over SSH. The animation extension (`a=f`) is Kitty-only: Ghostty and WezTerm don't implement it. |
| Frame deltas | Not in this round | A plot frame is 10–30 KB as PNG and encodes in about 10 ms; a sliding window changes every pixel anyway. Deltas need the animation extension plus a fallback, and dirty-rectangle tracking in the canvas. |
| Where the cursor is between frames | On the line below the image, at column 1 | An interrupted program leaves the terminal usable. The caller must not write to the terminal while a plot is live. |
| Image ids | A process-wide counter hands each placement a 16-bit block below 2^24, seeded from the process id; frames use increasing ids within the block | Two processes sharing one terminal (tmux panes) seldom collide. Overlapping images with the same z-index stack by id, so the new frame is on top for the instant both exist. Ids below 2^24 keep Unicode placeholders possible later. |
| Frame pacing | The caller's job; the library never sleeps | A library can't know the data rate. The CLI paces with `--interval`. |
| Automatic limits | Follow the data every frame | matplotlib's autoscale. Explicit limits or `Series::keep_last` give stable axes. |
| `LegendLocation::Best` | Re-scored every frame | matplotlib does the same on every draw. A fixed `legend_location` stops the legend moving. |
| Data API | `&mut` access to existing series on `Plot` and `Graph`, plus `push`, `extend`, `clear`, `keep_last` and `data_mut` on `Series` | Appending a point must not clone the plot. Adding series still goes through the builders. |
| Resize, keys, placeholders | Not handled | The frame size is fixed by the first frame. Ctrl-C ends the CLI through the default signal handling; no raw mode. tmux still loses the image on a redraw, as today. |

## Architecture

```
Plot::show_live()  ──►  LivePlot { placement, width, height, font_size }
   update(&Plot)          canvas_with_font_size(..).draw() → PNG → placement.replace()
                                         │
Terminal::place(&Image) ──►  Placement { terminal, rows, width, height, ids }
   replace(&Image)            frame_bytes(..) → one write_all + flush
```

| Unit | Responsibility | Depends on |
|---|---|---|
| `plotting/series.rs` | `push`, `extend`, `clear`, `keep_last`, `data_mut` | none |
| `plotting/graph.rs`, `plot.rs` | `Graph::data_mut()`, `Plot::series_mut()` returning `&mut [Series]` | `series.rs` |
| `kitty_graphics/ctrl_seq.rs` | `Action::{Transmit, Put, Delete}`, a delete-target key (`d=I`), the existing `Metadata::Id` | none |
| `terminal_commands/kitty_cmds.rs`, `images.rs` | Building the transmit, put and delete commands as bytes (control-only commands carry no payload and no `m` key); wrapping for tmux | `ctrl_seq.rs` |
| `terminal_commands/csi_cmds.rs` | Cursor up and down by *n* rows, and carriage return, as bytes | none |
| `terminal.rs` | `Placement`, `Terminal::place`, `frame_bytes` (pure), the id allocator, `Error::ImageTooTall`, `Error::PlacementSize` | the three above |
| `plot.rs` | `LivePlot`, `Plot::show_live`, `Plot::show_live_in` | `terminal.rs` |
| CLI `data.rs` | A line-at-a-time table reader with the same header, delimiter, missing-value and column rules as `Table::parse` | none |
| CLI `main.rs`, `cli.rs` | `--follow`, `--interval`, `--window`; the reader thread and the frame loop; deferred warnings | `LivePlot`, `data.rs` |
| `tests/pty.rs` | Piped stdin with timed writes, decoding of several graphics commands and of cursor moves | none |

## Public API

All additive; nothing breaks. Version 0.4.0.

```rust
impl Series {
    /// Appends a point.
    pub fn push<X: Graphable, Y: Graphable>(&mut self, x: X, y: Y);
    /// Appends points.
    pub fn extend<X: Graphable, Y: Graphable>(&mut self, points: impl IntoIterator<Item = (X, Y)>);
    /// Removes every point; the styles and label stay.
    pub fn clear(&mut self);
    /// Drops the oldest points so that at most `n` remain (a sliding window).
    pub fn keep_last(&mut self, n: usize);
    /// The points, for changes the methods above don't cover (`retain`, `drain`, ...).
    pub fn data_mut(&mut self) -> &mut Vec<Point<f64>>;
}

impl Graph {
    /// The series, for changing their data or styles in place.
    pub fn data_mut(&mut self) -> &mut [Series];
}

impl Plot {
    /// The series, in the order they were added, for changing their data or styles in place.
    pub fn series_mut(&mut self) -> &mut [Series];
    /// Draws the plot in the terminal and returns a handle for updating it in place.
    pub fn show_live(&self) -> Result<LivePlot>;
    /// Like `show_live`, with a `Terminal` that was already connected.
    pub fn show_live_in(&self, terminal: &Terminal) -> Result<LivePlot>;
}

/// A plot shown in the terminal that can be redrawn in place. Dropping it leaves the last
/// frame on screen with the cursor below it.
pub struct LivePlot { /* placement, width, height, font_size */ }

impl LivePlot {
    /// Redraws `plot` over the previous frame, at the size and text size of the first frame.
    pub fn update(&mut self, plot: &Plot) -> Result<()>;
    /// The frame size in pixels.
    pub fn size(&self) -> (u32, u32);
}

// terminal
impl Terminal {
    /// Shows `image` at the cursor's line, column 1, and returns a handle for replacing it.
    pub fn place(&self, image: &Image) -> Result<Placement>;
}

/// An image shown at a fixed place in the terminal, which can be replaced in place.
pub struct Placement { /* terminal, rows, width, height, block, frame */ }

impl Placement {
    /// Replaces the image; `image` must have the placement's pixel size.
    pub fn replace(&mut self, image: &Image) -> Result<()>;
    /// PNG-encodes RGB8 pixels and replaces the image, like `Terminal::show_rgb`.
    pub fn replace_rgb(&mut self, rgb: &[u8], width: u32, height: u32) -> Result<()>;
    /// The size in pixels.
    pub fn size(&self) -> (u32, u32);
    /// The terminal rows the image covers.
    pub fn rows(&self) -> u32;
}

// error
pub enum Error {
    // ...
    /// A placement's image covers more rows than the window has, less one for the cursor.
    ImageTooTall { rows: u32, screen_rows: u32 },
    /// A replacement image does not have the placement's size.
    PlacementSize { expected: (u32, u32), got: (u32, u32) },
}
```

`LivePlot` and `Placement` are `Send + Sync` (they hold a `Terminal`, which is `Copy`, and
integers), so a program can render on one thread and read data on another.

```rust
let mut plot = Plot::new().line(Series::from(vec![(0.0, 0.0)]).with_label("v"));
let mut live = plot.show_live()?;                 // first frame, sized to the terminal
for (t, v) in samples {
    plot.series_mut()[0].push(t, v);
    plot.series_mut()[0].keep_last(500);          // sliding window
    live.update(&plot)?;
}
```

Changing anything else about the plot between frames (limits, title, another series) goes
through the by-value builders: `plot = plot.x_limits(a, b)`. `LivePlot` doesn't borrow the plot.

## Frames on the terminal

`Placement` writes one buffer per frame with a single `write_all` and flush, so an interruption
between frames always leaves the cursor below the image. Rows are `rows_covered(height)`, as
the tmux path of `Terminal::show` computes them. `n` below is that row count.

**Placing** (`Terminal::place`):

```
\r  \n × n        reserve n rows below the cursor's line, scrolling if needed
CSI n A           back up to the image's first row
APC a=t,i=ID,f=100,q=2[,m=1] ; <png chunk> ...  APC ... m=0 ; <last chunk>
APC a=p,i=ID,C=1,q=2          display at the cursor without moving it
CSI n B           down to the line below the image
```

**Replacing** (`Placement::replace`):

```
APC a=t,i=NEW,f=100,q=2 chunks   the new frame, while the old one stays visible
CSI n A
APC a=p,i=NEW,C=1,q=2            the new frame over the old (higher id, so on top)
APC a=d,d=I,i=OLD,q=2            the old frame and its data
CSI n B
```

- `a=p` and `a=d` are control-only commands: no payload and no `m` key.
- In tmux, only the APC commands are wrapped in passthrough (each chunk on its own, as now);
  the cursor moves and newlines are written plain, so tmux tracks the cursor.
- Before placing, `rows_covered(height) > window.rows - 1` fails with `ImageTooTall`: the image
  must fit on the screen with a line left for the cursor. Images wider than the window are
  clipped on the right, as they are today.
- The image starts at column 1 (the `\r`). `Terminal::show` keeps drawing at the cursor column.
- **Ids.** A process-wide `AtomicU32` counter starts at the process id. Each placement takes
  the next value and uses block `1 + (value % 255)`. Frame `k` (from 1) has id
  `block << 16 | (k % 65536)`. Ids are never 0, stay below 2^24, and rise within a placement;
  after 65535 frames they wrap, and the wrapped frame sits under the previous one only for the
  instant between the put and the delete in the same write.
- Both terminal query paths are untouched: a live plot sends no queries after `connect`.
- On a write error (`EPIPE`, terminal gone) `replace` returns `Error::Io`; nothing is retried.

**Why not the animation extension.** `a=f` can send only the changed rectangle of a frame,
which is what issue #21 imagined. It's Kitty-only, so the whole-frame path is needed anyway,
and the saving is small for plots: a sliding window changes every pixel, and an append-only
plot with fixed axes still needs dirty-rectangle tracking through the canvas, legend and text.
The `Placement` API leaves room for it: a later `replace` could diff the previous RGB buffer
and send a frame delta when the terminal is known to be Kitty.

## Rendering per frame

`LivePlot::update` runs exactly what `show_in` runs: `canvas_with_font_size(width, height,
font_size).draw()?.into_bytes()`, then `Image::png_from_rgb`, then `Placement::replace`. The
size is the plot's or the terminal's default, and the text size is the plot's or the
terminal's, both decided in `show_live` and kept. A whole frame at 1200x600 takes about 10–15 ms
(layout and drawing a few ms, PNG encoding about 10 ms), which allows 30 frames per second
locally; over SSH the 10–30 KB per frame is the limit, and the CLI's default interval of 100 ms
keeps that at 100–300 KB/s.

Nothing is cached between frames. Ticks, labels and the legend are recomputed, so the axes
follow the data as with matplotlib's autoscale. If profiling later shows the layout matters,
a layout cache keyed on the view limits, size and font is the obvious step.

## CLI

**Flags** (help heading "Live"):

| Flag | Effect |
|---|---|
| `-f, --follow` | Keep reading stdin as lines arrive and update the plot in place, like `tail -f`. Stdin is always a data source in this mode (`-` need not be given). Files, `--data` and `--series` without `file=-` are static series drawn on every frame. Requires a terminal: with `--output` it's an error, and without piped stdin it's an error. |
| `--interval <MS>` | Redraw at most every MS milliseconds [default: 100]. `0` redraws whenever new points have arrived. Requires `--follow`. |
| `--window <N>` | Keep only the last N points of each streamed series [default: all]. Requires `--follow`. |

**Reading.** A thread reads stdin with `BufRead::lines` and sends each line over a channel.
The main loop waits on the channel with `recv_timeout` until the next frame is due, appends the
points from every line it gets, applies `--window`, and redraws when points arrived and the
interval has passed since the last frame. Stdin closing ends the loop after a final frame.

**Parsing.** The line-at-a-time reader keeps `Table::parse`'s rules: a UTF-8 byte order mark on
the first line is skipped, blank and `#` lines are ignored, the first remaining line is a header
when any field is neither a number nor a missing value, fields split on commas, else
semicolons, else tabs, else whitespace, with quotes stripped. Columns are resolved once the
header (or the first data row, for a file without one) is known, with today's messages;
`-y name` without a header is the same error as now. A single-column stream is plotted against
the row number, decided from the header or first data row. The first frame is drawn as soon as
the first point exists; a stream that ends without one is the existing "no data points found in
stdin" error. Malformed rows are errors, as now: the message is printed below the plot and the
program exits with status 1.

**Warnings.** Missing values and non-finite points are skipped as today, but printing a warning
would scroll the plot, so in follow mode they are counted and one line is printed below the
plot when the stream ends (`warning: skipped 3 row(s) with missing values and 1 point(s) with
NaN or infinite values in stdin`). `--verbose` lines print before the first frame only.

**Ending.** At the end of stdin: the final frame, the summary line if any, exit 0. On Ctrl-C the
default signal handling ends the process; the last frame stays and the cursor is already below
it. Nothing enables raw mode, and keys are not read.

**Legend and names.** As for a static plot: the streamed series are named from the header, else
`stdin`, and the legend shows for 2 or more series unless the legend flags say otherwise.

## Testing

**Unit tests**
- `frame_bytes`: exact bytes for placing (with 1 and with 3 chunks) and replacing, for
  `Passthrough::None` and `Passthrough::Tmux` (only APC commands wrapped, escapes doubled),
  `q=2` and `C=1` on every put, `d=I` naming the previous id, cursor moves equal to `rows`.
- Ids: blocks come from the counter, ids are non-zero and below 2^24, rise within a placement,
  and wrap after 65535 frames.
- `Placement`: `replace` with a different size is `PlacementSize`; an image taller than
  `rows - 1` is `ImageTooTall`; `rows()` matches `rows_covered`.
- `LivePlot` and `Placement` against a byte sink: `Placement` writes through an injected
  `Write` (stdout in `Terminal::place`, a `Vec<u8>` in tests), so two updates are checked to send
  two transmissions and one delete, and the second frame's size to equal the first's even when
  the plot's size changed in between.
- `Series`: `push`/`extend` convert like `Series::from`; `keep_last` keeps the newest points and
  is a no-op when `n >= len`; `clear` keeps the label and styles; `data_mut` round-trips.
- CLI: the line reader against every rule above with the same inputs as the `Table::parse`
  tests; `--interval` and `--window` require `--follow`; `--follow` with `--output` errors.

**Property tests** (`tests/properties.rs`): a random sequence of `push`, `keep_last`, `clear`
and `extend` on random series, rendered after each step, never panics, and every rendered frame
has `width * height * 3` bytes.

**Golden images**: unchanged. The pipeline for a frame is the pipeline for a PNG.

**pty tests** (`tests/pty.rs`, after the harness work):
- `--follow` with three lines written 50 ms apart and `--interval 0`: three images, each a
  decodable PNG of the same size; the first command of each frame after the first is `a=t`
  with a new id; every put has `C=1` and `q=2`; every delete names the previous frame's id; the
  cursor moves up and down `rows` around each put; nothing follows the last frame's cursor-down
  (no summary line here), because the cursor is already below the image.
- A line with a missing value: one frame fewer, and the summary line after the stream ends.
- `--follow` in tmux: every APC wrapped, cursor moves unwrapped.
- Today's CLI with piped stdin inside a pty (the harness addition): one image, queries answered
  on the tty.

**Manual checklist** (in the PR description): Kitty, Ghostty, WezTerm; inside tmux; over SSH with
a slow link; a plot whose data grows past the axes (rescale) and one with fixed limits; Ctrl-C
mid-stream leaves the cursor below the plot.

## Docs

- **README:** a "Live plots" library example (the loop above) and a CLI "Follow mode" section
  with `tail -f` and `--window`/`--interval`; rows for the three flags in the options table;
  a note that the plot area must not be written to while live.
- **CLAUDE.md:** `Placement`/`LivePlot` in the public API layers, the frame format under the
  Kitty protocol section, the follow loop under CLI, and the pty harness additions under
  Testing.
- **CHANGELOG.md**, under `[0.4.0] - Unreleased`: Added (`Series::push`, `extend`, `clear`,
  `keep_last`, `data_mut`; `Graph::data_mut`, `Plot::series_mut`; `Plot::show_live`,
  `LivePlot`; `Terminal::place`, `Placement`; `Error::ImageTooTall`, `Error::PlacementSize`;
  CLI `--follow`, `--interval`, `--window`).
- **`examples/live.rs`:** appends a sine sample every 50 ms for 10 s with a window of 200
  points (`cargo run --example live`).

## Delivery

Four preparatory PRs, independent of each other and mergeable in any order, then two feature
PRs. Each is green on its own (tests, clippy, fmt, docs, MSRV) and opened against `main`.

| PR | Issue | Content |
|---|---|---|
| Prep 1 | #47 Series, Graph and Plot mutation API | `push`, `extend`, `clear`, `keep_last`, `data_mut`, `Graph::data_mut`, `Plot::series_mut`, tests, CHANGELOG lines |
| Prep 2 | #48 Protocol building blocks | `a=t`/`a=p`/`a=d` actions and `d=I`, image ids on transmit/put/delete commands, control-only commands, command bytes without writing them, cursor up/down builders, exact-bytes tests; no behavior change |
| Prep 3 | #49 CLI streaming table reader | Line-at-a-time reader and per-row point conversion with today's rules; `Table::parse` + `points` reimplemented on top, existing tests unchanged |
| Prep 4 | #50 pty harness for streams | Piped stdin with timed writes while the tty is the pty; decoding of several graphics commands and of cursor moves; a test of today's CLI with piped stdin inside a pty |
| Feature A | #51 Library live plots | `Placement`, `frame_bytes`, ids, `LivePlot`, `show_live`, errors, `examples/live.rs`, property test, README library section, CLAUDE.md, CHANGELOG. Needs #47 and #48. |
| Feature B | #52 CLI follow mode | `--follow`, `--interval`, `--window`, reader thread, deferred warnings, pty tests, README CLI section, CLAUDE.md, CHANGELOG. Needs #51, #49 and #50. |

0.4.0 is tagged after feature B, when the user asks.

## Out of scope

- Frame deltas through the animation extension (`a=f`), and Unicode placeholders (`U=1`) for
  tmux redraws.
- Handling a terminal resize during a live plot; keyboard control (`q` to quit); raw mode.
- `--follow` on a growing file, a window by x span (`--window-x`), and several streamed
  sources.
- Pinning `LegendLocation::Best` on the first frame.
- Deleting a placement (`Placement::delete`), and placements at a position other than the
  cursor's line.
- GIF or APNG export of an animation.
- A layout cache between frames.
