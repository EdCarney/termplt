# Subplots (Figure) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. If those skills are not installed, work through the tasks in order, one test cycle at a time, exactly as written.

**Goal:** A `Figure` that draws several `Plot`s as one image on a grid (with spanning cells), rendered, saved, shown or shown live, plus empty axes for plots without data.

**Architecture:** Each cell's `Plot` is rendered on its own at its cell's size through the existing single-plot pipeline and copied into the figure's RGB buffer; nothing in the canvas layout changes. Live figures reuse `LivePlot` and `Placement` (whole-image replacement) through a sealed `Render` trait. Empty axes come from `Graph::view_limits` returning 0–1 when no point is visible.

**Tech Stack:** Rust 2024, MSRV 1.88; `image` (PNG), `rgb`; tests with `proptest`, `tempfile` and PNG golden snapshots. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-10-04-multi-plot-design.md`. Read it before starting; this plan argues from it. Read `CLAUDE.md` too: it describes the crate, its commands and its conventions.

## Global Constraints

- No new dependencies; MSRV stays `rust-version = "1.88"` (no std APIs newer than 1.88).
- Every public item is documented (`#![warn(missing_docs)]`; `cargo doc` runs with `-D warnings`).
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --all -- --check` must be clean.
- Every existing golden snapshot in `tests/snapshots/` must still match, byte for byte as far as the 0.1% tolerance goes: never regenerate an existing one. Only new snapshots are created.
- `Plot::show`, `save_png`, `render` and `LivePlot` frames for a plot with data produce the same bytes as before.
- Behaviour follows matplotlib unless the spec says otherwise (empty axes 0–1, `best` legend upper right when nothing is plotted).
- Tests use `tempfile` for files, never fixed names in the shared temp directory.
- Comments and docs follow the crate's style: short, plain sentences.

## Review Focus

Inputs the spec implies but its examples don't exercise; each has a test in the task that owns the code.

1. **Huge grids**, such as `Figure::new(usize::MAX, usize::MAX)` with one plot. Expect validation and rendering cost proportional to the number of plots, not slots: no allocation per slot and no hang. Validation is pairwise and slots are computed per index. Tested in Tasks 3 and 4.
2. **Spans at the integer limits:** `usize::MAX`, `0..=usize::MAX`, `1..` on a huge grid. Expect `InvalidCell` or a valid range, never an overflow panic. Tested in Task 3.
3. **More columns or rows than pixels.** Expect a cell of 0 pixels to fail with `CanvasTooSmall`, not panic or draw out of bounds. Tested in Task 4.
4. **A size set on a plot inside a figure.** Expect it to be ignored: the cell size wins. Tested in Task 4.
5. **A live figure whose grid shape changes between frames,** or that switches between a plot and a figure. Expect every frame to keep the first frame's size. Tested in Task 6.

## Working on this plan

- **Start from branch `multi-plot-spec`** (on `origin`). It holds the spec and this plan, and is `main` plus those two documents.
- **Three milestones, one PR each, in order:**

  | Milestone | Tasks | Branch | PR base |
  |---|---|---|---|
  | 1. Empty axes | 1 | `multi-plot-empty-axes`, from `multi-plot-spec` | `main` (it carries the spec and plan too) |
  | 2. Static figures | 2–5 | `multi-plot-figure`, from milestone 1 | `multi-plot-empty-axes` |
  | 3. Live figures | 6 | `multi-plot-live`, from milestone 2 | `multi-plot-figure` |

  Each PR description says which spec section and tasks it implements. A stacked PR says "stacked on #N; merge in order". If your environment only lets you push one branch, do the milestones in order on that branch. Push after each milestone, and tell the user it's ready for a PR with a title and description they can use.
- **At the end of every milestone,** all of these pass. CI runs the same on Linux, macOS and Windows.

  ```bash
  cargo fmt --all -- --check
  cargo clippy --all-targets -- -D warnings
  cargo test
  cargo test --no-default-features
  RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
  cargo +1.88 test --locked   # if the toolchain is missing: rustup toolchain install 1.88
  ```
- **New golden snapshots:** create them with `TERMPLT_UPDATE_SNAPSHOTS=1 cargo test --test golden <test name>`. Then open each new PNG and check it shows what the task says. `git status` must show only new files under `tests/snapshots/`.
- **Commits:** one per task at least; messages in the imperative, like the existing history ("Add …", "Draw …").

---

## Milestone 1: Empty axes

Branch `multi-plot-empty-axes` from `multi-plot-spec` (`git switch -c multi-plot-empty-axes origin/multi-plot-spec`).

### Task 1: Plots without data draw empty axes

**Files:**
- Modify: `src/plotting/graph.rs`: `visible`, `view_limits`, their docs, tests
- Modify: `src/plotting/canvas.rs`: `TerminalCanvas::layout_with` (the `largest_marker_sz` lookup), test `empty_graph_returns_error`
- Modify: `src/plot.rs`: tests
- Modify: `src/error.rs`: docs of `NoData` and `NoVisibleData`
- Modify: `tests/golden.rs`; create `tests/snapshots/empty_plot.png`
- Modify: `CHANGELOG.md`, `CLAUDE.md`

**Interfaces:**
- Produces: `Graph::view_limits()` returns `Ok` when no point is visible: each axis without explicit limits is `0.0..1.0`, each axis with them uses them. `Graph::scale` and `Graph::scale_with_view` return graphs whose series are empty instead of failing with `NoVisibleData`. `TerminalCanvas::draw` and `Plot::render` draw empty axes for a graph without series or points. `Graph::limits()` and `Drawable::get_mask` for `Graph` are unchanged and still fail with `NoData`.

- [ ] **Step 1: Write the failing graph tests** (in `graph.rs`'s `mod tests`; `graph_with_data()` already exists there)

```rust
fn unit_square() -> Limits<f64> {
    Limits::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0))
}

#[test]
fn view_limits_without_data_are_zero_to_one() {
    assert_eq!(Graph::new().view_limits().unwrap(), unit_square());
    let empty = Graph::new().with_series(Series::new::<f64>(&[]));
    assert_eq!(empty.view_limits().unwrap(), unit_square());
    let non_finite = Graph::new().with_series(Series::new(&[
        Point::new(f64::NAN, 1.0),
        Point::new(2.0, f64::INFINITY),
    ]));
    assert_eq!(non_finite.view_limits().unwrap(), unit_square());
}

#[test]
fn view_limits_without_data_keep_explicit_limits() {
    let view = Graph::new().with_x_limits(2, 5).view_limits().unwrap();
    assert_eq!(view, Limits::new(Point::new(2.0, 0.0), Point::new(5.0, 1.0)));
}

#[test]
fn view_limits_with_every_point_clipped_are_the_explicit_limits() {
    let view = graph_with_data().with_x_limits(100, 200).view_limits().unwrap();
    assert_eq!(view, Limits::new(Point::new(100.0, 0.0), Point::new(200.0, 1.0)));
}

#[test]
fn invalid_limits_fail_without_data_too() {
    let err = Graph::new().with_x_limits(5, 1).view_limits().unwrap_err();
    assert!(matches!(err, Error::InvalidLimits { axis: "x", .. }), "{err}");
}

#[test]
fn scale_without_data_keeps_empty_series() {
    let g = Graph::new().with_series(Series::new::<f64>(&[]));
    let new_limits = Limits::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
    let scaled = g.scale(new_limits).unwrap();
    assert_eq!(scaled.data().len(), 1);
    assert!(scaled.data()[0].data().is_empty());
}
```

Replace the existing test `scale_with_limits_excluding_all_points_returns_error` with:

```rust
#[test]
fn scale_with_limits_excluding_all_points_keeps_empty_series() {
    let g = graph_with_data().with_x_limits(100, 200);
    let new_limits = Limits::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
    let scaled = g.scale(new_limits).unwrap();
    assert!(scaled.data().iter().all(|s| s.data().is_empty()));
    assert_eq!(scaled.x_limits(), Some((0.0, 100.0)));
}
```

Keep `limits_with_no_data_returns_error` and `get_mask_on_empty_graph_returns_error` unchanged.

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test --lib plotting::graph`
Expected: five of the new tests FAIL (with `NoData` or `NoVisibleData`).
`invalid_limits_fail_without_data_too` already passes and must keep passing, as must every
other graph test.

- [ ] **Step 3: Implement empty view limits in `src/plotting/graph.rs`**
  - **`fn visible(&self) -> Result<Graph>`:** call `self.validate_limits()?` instead of `self.limits()?`. Keep a point when it is finite and inside each explicit range that is set (inclusive, like `Limits::contains`; `Option::is_none_or` is fine on 1.88). This keeps exactly the points kept today, and a graph without finite points no longer fails.
  - **`pub fn view_limits(&self) -> Result<Limits<f64>>`:** when `self.visible()?.limits()` is `Err(Error::NoData)`, build the view from `x_limits`/`y_limits`, using `(0.0, 1.0)` for an axis without them. Pass it through `pad_degenerate` and the same finite-span check (`DataRangeTooLarge`) as today. Otherwise the code is today's. Remove the `map_err(|_| Error::NoVisibleData)`.
  - Add one sentence to the docs of `view_limits` and `scale`: without visible points, axes without explicit limits are 0 to 1, as matplotlib draws empty axes.

- [ ] **Step 4: Run the graph tests**

Run: `cargo test --lib plotting::graph`
Expected: PASS.

- [ ] **Step 5: Write the failing canvas and plot tests**

In `canvas.rs`'s tests, replace `empty_graph_returns_error` with (`white_axes()` is an existing helper there):

```rust
#[test]
fn an_empty_graph_draws_its_axes() {
    let canvas = TerminalCanvas::new(100, 100, colors::BLACK)
        .with_graph(Graph::new().with_axes(white_axes()));
    assert!(canvas.get_drawable_limits().is_ok());
    let bytes = canvas.draw().unwrap().into_bytes();
    assert!(bytes.iter().any(|&b| b != 0));
}
```

In `plot.rs`'s tests, replace `empty_plot_is_an_error` and `a_frame_that_cannot_be_drawn_sends_nothing` with these (`live_terminal()` and `count()` already exist there):

```rust
#[test]
fn an_empty_plot_draws_empty_axes() {
    for plot in [Plot::new(), Plot::new().line(Vec::<(f64, f64)>::new())] {
        let rgb = plot.render(200, 150).unwrap();
        assert!(rgb.iter().any(|&b| b != 0));
    }
}

#[test]
fn an_empty_labeled_series_has_its_legend_upper_right() {
    let series = Series::from(Vec::<(f64, f64)>::new()).with_label("waiting");
    let (w, h) = (400usize, 300usize);
    let with = Plot::new().line(series.clone()).render(400, 300).unwrap();
    let without = Plot::new().line(series).legend(false).render(400, 300).unwrap();
    let differing: Vec<(usize, usize)> = (0..w * h)
        .filter(|i| with[i * 3..i * 3 + 3] != without[i * 3..i * 3 + 3])
        .map(|i| (i % w, i / w)) // (column, row from the top)
        .collect();
    assert!(!differing.is_empty());
    assert!(differing.iter().all(|&(x, y)| x > w / 2 && y < h / 2));
}

#[test]
fn a_frame_that_cannot_be_drawn_sends_nothing() {
    let terminal = live_terminal();
    let inverted = Plot::new().line(vec![(0, 0), (1, 1)]).x_limits(1.0, 0.0);
    assert!(matches!(
        inverted.show_live_with(&terminal, Terminal::place_in_buffer),
        Err(crate::Error::InvalidLimits { .. })
    ));

    let plot = Plot::new().line(vec![(0, 0), (1, 1)]);
    let mut live = plot
        .show_live_with(&terminal, Terminal::place_in_buffer)
        .unwrap();
    let written = live.placement.written().len();
    assert!(matches!(
        live.update(&inverted),
        Err(crate::Error::InvalidLimits { .. })
    ));
    assert_eq!(live.placement.written().len(), written);
    // the next frame that can be drawn replaces the first one
    live.update(&plot).unwrap();
    assert_eq!(count(live.placement.written(), b"a=d,"), 1);
}

#[test]
fn an_empty_live_plot_shows_empty_axes() {
    let mut plot = Plot::new().line(Series::from(Vec::<(f64, f64)>::new()).with_label("v"));
    let mut live = plot
        .show_live_with(&live_terminal(), Terminal::place_in_buffer)
        .unwrap();
    plot.series_mut()[0].push(1, 1);
    live.update(&plot).unwrap();
    plot.series_mut()[0].clear();
    live.update(&plot).unwrap();
    assert_eq!(count(live.placement.written(), b"a=t,"), 3);
}
```

- [ ] **Step 6: Run them and see them fail**

Run: `cargo test --lib -- canvas::tests::an_empty_graph_draws_its_axes plot::tests`
Expected: `an_empty_graph_draws_its_axes` and `an_empty_plot_draws_empty_axes` FAIL with `NoData`, which comes from `layout_with` when the graph has no series at all. The tests with an empty series already pass after Step 3, as do the other plot tests.

- [ ] **Step 7: In `TerminalCanvas::layout_with`, use 0 for the largest marker or line size when the graph has no series**

Replace `.max().ok_or(Error::NoData)?` with `.max().unwrap_or(0)`. Series that exist but are empty still count, as today. Remove the `Error` import if it becomes unused.

- [ ] **Step 8: Run the library tests**

Run: `cargo test --lib`
Expected: PASS.

- [ ] **Step 9: Add the golden scene** to `tests/golden.rs`, after `legend_light_background`, using its existing `axes()`, `grid()` and `check` helpers

```rust
#[test]
fn empty_plot() {
    let (w, h) = (320, 240);
    let waiting = Series::new::<f64>(&[])
        .with_marker_style(MarkerStyle::None)
        .with_line_style(LineStyle::Solid {
            color: colors::DODGER_BLUE,
            thickness: 0,
        })
        .with_label("waiting for data");
    let canvas = TerminalCanvas::new(w, h, colors::BLACK)
        .with_buffer(BufferType::Uniform(8))
        .with_graph(
            Graph::new()
                .with_series(waiting)
                .with_axes(axes())
                .with_grid_lines(grid())
                .with_title("No data yet"),
        );
    check("empty_plot", w, h, canvas);
}
```

Run `TERMPLT_UPDATE_SNAPSHOTS=1 cargo test --test golden empty_plot`, then `cargo test --test golden`. Expected: all pass. Open `tests/snapshots/empty_plot.png`. It must show:
- the title;
- both axes labeled `0.0` to `1.0` in steps of `0.2`, with the grid at those ticks;
- in the upper right, a legend with a blue line sample and "waiting for data".

- [ ] **Step 10: Docs**
  - **`src/error.rs`:** `NoData` is now returned only by `Graph::limits` and `Graph::get_mask`, and drawing a plot doesn't need data. `NoVisibleData` is no longer returned and is kept so code matching it still compiles. Update the `Display` text only if it stops being accurate.
  - **`CHANGELOG.md`:** add `## [Unreleased]` above `## [0.4.0]`, with a `### Changed` entry. A plot without data now draws empty axes (0 to 1 on each axis without explicit limits, as matplotlib draws empty axes) instead of failing with `Error::NoData`. Points all outside explicit limits now draw the axes at those limits instead of failing with `Error::NoVisibleData`, which is no longer returned. Live plots can start empty.
  - **`CLAUDE.md`:** under "Zero-span safety", add one sentence. When no point is visible, `view_limits()` is 0–1 on axes without explicit limits (matplotlib's empty axes), and `layout_with` uses no marker inset for a graph without series.

- [ ] **Step 11: Milestone checks, commit, push, PR**

Run the six commands from "Working on this plan". Expected: all pass, and `git status` shows `tests/snapshots/empty_plot.png` as the only new snapshot.

```bash
git add -A src tests CHANGELOG.md CLAUDE.md
git commit -m "Draw empty axes for plots without data"
git push -u origin multi-plot-empty-axes
```

Open a PR against `main`, titled "Draw empty axes for plots without data". Its description says it implements the spec's "Empty axes" section and carries the spec and plan.

---

## Milestone 2: Static figures

Branch `multi-plot-figure` from `multi-plot-empty-axes`.

### Task 2: Plots take figure defaults

**Files:**
- Modify: `src/plot.rs`

**Interfaces:**
- Produces (crate-private, used by Task 4):

```rust
/// What a figure gives the plots in it: each plot uses its own setting where it has one.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Defaults<'a> {
    pub(crate) background: RGB8,
    pub(crate) font: &'a Font,
    pub(crate) font_size: u32,
}

impl Plot {
    /// Draws the plot at `width` x `height` pixels, using `defaults` for the background, font
    /// and base text size where the plot has none of its own. A size set on the plot is ignored.
    pub(crate) fn render_with(&self, width: u32, height: u32, defaults: Defaults<'_>) -> Result<Vec<u8>>;
}
```

- [ ] **Step 1: Write the failing tests** (in `plot.rs`'s tests)

```rust
#[test]
fn figure_defaults_apply_where_the_plot_has_none() {
    let plot = Plot::new().line(vec![(0, 0), (1, 1)]);
    let font = Font::default();
    let defaults = Defaults { background: colors::WHITE, font: &font, font_size: 20 };
    let expected = plot.clone().background(colors::WHITE).font_size(20);
    // dark axes on the white background, as for a plot that set it
    assert!(plot.render_with(320, 240, defaults).unwrap() == expected.render(320, 240).unwrap());
}

#[test]
fn a_plots_own_settings_win_over_figure_defaults() {
    let plot = Plot::new()
        .line(vec![(0, 0), (1, 1)])
        .background(colors::BLACK)
        .font_size(12)
        .size(50, 50); // ignored: the size given wins
    let font = Font::default();
    let defaults = Defaults { background: colors::WHITE, font: &font, font_size: 20 };
    assert!(plot.render_with(320, 240, defaults).unwrap() == plot.render(320, 240).unwrap());
}

#[test]
fn unset_and_default_settings_draw_alike() {
    let plot = Plot::new().line(vec![(0, 0), (1, 1)]);
    let explicit = plot.clone().background(colors::BLACK).font(Font::default());
    assert!(plot.render(320, 240).unwrap() == explicit.render(320, 240).unwrap());
    assert_ne!(plot, explicit); // "set to the default" and "not set" differ now
}
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test --lib plot::tests`
Expected: compile error, `Defaults` and `render_with` don't exist.

- [ ] **Step 3: Implement in `src/plot.rs`**
  - **Fields:** `background: Option<RGB8>` (`None` draws black) and `font: Option<Font>` (`None` is the built-in font); the setters store `Some`.
  - **Graph:** move `graph()`'s body into a private `fn graph_on(&self, background: RGB8) -> Graph`, which picks the foreground and grid colors from that background. `graph()` calls it with `self.background.unwrap_or(colors::BLACK)`.
  - **Canvas:** a private `fn canvas_with(&self, width: u32, height: u32, font_size: u32, background: RGB8, font: Font) -> TerminalCanvas` holds today's `canvas_with_font_size` body. `canvas_with_font_size` calls it with the plot's own background and font, or black and `Font::default()`, so every existing path draws the same bytes.
  - **`render_with`:** `canvas_with(width, height, own font_size or defaults.font_size, own background or defaults.background, own font or defaults.font.clone())`, then `.draw()?.into_bytes()`.
  - Update the doc comments of `background`, `font` and `font_size` to say a figure's setting applies to plots that don't set their own.

- [ ] **Step 4: Run the library tests**

Run: `cargo test --lib`
Expected: PASS, including the existing `plot::tests` such as `light_background_uses_dark_axes` and `a_plot_updated_in_place_renders_like_one_built_from_scratch`.

- [ ] **Step 5: Commit**

```bash
git add src/plot.rs
git commit -m "Let plots take a background, font and text size from a figure"
```

### Task 3: `Figure`, `GridSpan` and grid validation

**Files:**
- Create: `src/figure.rs`
- Modify: `src/error.rs`: three variants, `Display`, tests
- Modify: `src/lib.rs`: `mod figure; pub use figure::{Figure, GridSpan};`
- Modify: `src/prelude.rs`: add `Figure`

**Interfaces:**
- Consumes: `Plot` (unchanged public API).
- Produces:

```rust
/// Several plots drawn as one image, on a grid of equal-size slots. A plot covers one slot or a
/// block of them. (Doc example added in Task 4, once it can render.)
#[derive(Debug, Clone, PartialEq)]
pub struct Figure {
    rows: usize,
    cols: usize,
    plots: Vec<Plot>,
    /// Rows and columns each plot covers, resolved; same order as `plots`.
    spans: Vec<(Range<usize>, Range<usize>)>,
    size: Option<(u32, u32)>,
    background: RGB8,      // default colors::BLACK
    font: Font,            // default Font::default()
    font_size: Option<u32>,
}

impl Figure {
    pub fn new(rows: usize, cols: usize) -> Figure;
    pub fn plot(self, rows: impl GridSpan, cols: impl GridSpan, plot: Plot) -> Self;
    pub fn size(self, width: u32, height: u32) -> Self;
    pub fn background(self, color: RGB8) -> Self;
    pub fn font(self, font: Font) -> Self;
    pub fn font_size(self, px: u32) -> Self;
    pub fn plots(&self) -> &[Plot];
    pub fn plots_mut(&mut self) -> &mut [Plot];
    pub(crate) fn validate(&self) -> Result<()>;
    /// (left, top, width, height) in pixels of plot `index`'s cell, rows counted from the top.
    /// Only for a validated figure.
    fn cell_rect(&self, index: usize, width: u32, height: u32) -> (u32, u32, u32, u32);
}

/// Pixel where slot `index` of `count` slots across `length` pixels starts.
fn slot_start(index: usize, count: usize, length: u32) -> u32;

/// Rows or columns a plot covers: one index or a range. Sealed.
pub trait GridSpan: sealed::Span {}
mod sealed {
    pub trait Span {
        /// The range covered in a grid dimension of `len` slots.
        fn resolve(self, len: usize) -> std::ops::Range<usize>;
    }
}
// Error (error.rs):
EmptyGrid { rows: usize, cols: usize },
InvalidCell { index: usize, rows: Range<usize>, cols: Range<usize>, grid: (usize, usize) },
CellsOverlap { first: usize, second: usize },
```

The doc comments for the methods and the three variants are in the spec's "Public API" section; copy them.

- [ ] **Step 1: Write the failing tests** (in `figure.rs`'s `mod tests` and `error.rs`'s tests)

```rust
// figure.rs
use super::{sealed::Span, *};

#[test]
fn spans_resolve_to_ranges() {
    assert_eq!(2usize.resolve(4), 2..3);
    assert_eq!((1usize..3).resolve(4), 1..3);
    assert_eq!((1usize..=2).resolve(4), 1..3);
    assert_eq!((1usize..).resolve(4), 1..4);
    assert_eq!((..2usize).resolve(4), 0..2);
    assert_eq!((..=2usize).resolve(4), 0..3);
    assert_eq!((..).resolve(4), 0..4);
    // no overflow at the limits; validation rejects these
    assert_eq!((0usize..=usize::MAX).resolve(4), 0..usize::MAX);
    assert_eq!(usize::MAX.resolve(4), usize::MAX..usize::MAX);
}

#[test]
fn plots_keep_the_order_they_were_added_in() {
    // unsuffixed literals and ranges are accepted, as for slice indexing
    let mut fig = Figure::new(2, 2)
        .plot(0, 1, Plot::new().title("b"))
        .plot(1, .., Plot::new().title("a"));
    assert_eq!(fig.plots()[0].graph().title(), Some("b"));
    fig.plots_mut()[1] = Plot::new().title("c");
    assert_eq!(fig.plots()[1].graph().title(), Some("c"));
    assert_eq!(fig.spans[1], (1..2, 0..2));
}

#[test]
fn slots_share_the_remainder_from_the_first() {
    let starts = |count: usize, length: u32| -> Vec<u32> {
        (0..=count).map(|i| slot_start(i, count, length)).collect()
    };
    assert_eq!(starts(3, 100), [0, 34, 67, 100]);
    assert_eq!(starts(2, 7), [0, 4, 7]);
    assert_eq!(starts(4, 2), [0, 1, 2, 2, 2]);
    assert_eq!(slot_start(1, usize::MAX, 10), 1);
}

#[test]
fn a_spanning_plot_covers_the_union_of_its_slots() {
    let fig = Figure::new(3, 3).plot(1.., 0..2, Plot::new());
    assert_eq!(fig.cell_rect(0, 100, 100), (0, 34, 67, 66));
}

#[test]
fn a_grid_without_rows_or_columns_is_an_error() {
    assert!(matches!(Figure::new(0, 2).validate(), Err(Error::EmptyGrid { rows: 0, cols: 2 })));
    assert!(matches!(Figure::new(2, 0).validate(), Err(Error::EmptyGrid { rows: 2, cols: 0 })));
}

#[test]
fn a_plot_outside_the_grid_or_covering_nothing_is_an_error() {
    let fig = Figure::new(2, 2).plot(0, 0, Plot::new()).plot(1, 1..3, Plot::new());
    match fig.validate() {
        Err(Error::InvalidCell { index: 1, rows, cols, grid: (2, 2) }) => {
            assert_eq!((rows, cols), (1..2, 1..3));
        }
        other => panic!("{other:?}"),
    }
    for fig in [
        Figure::new(2, 2).plot(1..1, 0, Plot::new()),
        Figure::new(2, 2).plot(0..=usize::MAX, 0, Plot::new()),
        Figure::new(2, 2).plot(usize::MAX, 0, Plot::new()),
    ] {
        assert!(matches!(fig.validate(), Err(Error::InvalidCell { index: 0, .. })));
    }
}

#[test]
fn plots_sharing_a_slot_are_an_error() {
    let fig = Figure::new(2, 2)
        .plot(0, 0, Plot::new())
        .plot(1, 1, Plot::new())
        .plot(.., 1, Plot::new()); // covers (0, 1) and (1, 1)
    assert!(matches!(fig.validate(), Err(Error::CellsOverlap { first: 1, second: 2 })));
}

#[test]
fn valid_grids_pass() {
    assert!(Figure::new(2, 2).validate().is_ok()); // no plots: drawn as background
    let fig = Figure::new(2, 2)
        .plot(0, 0, Plot::new())
        .plot(0, 1, Plot::new())
        .plot(1, .., Plot::new());
    assert!(fig.validate().is_ok());
}

#[test]
fn huge_grids_cost_nothing_per_slot() {
    let fig = Figure::new(usize::MAX, usize::MAX)
        .plot(0, 0, Plot::new())
        .plot(1.., 1.., Plot::new());
    assert!(fig.validate().is_ok());
}

// error.rs
#[test]
fn figure_error_messages() {
    let empty = Error::EmptyGrid { rows: 0, cols: 2 };
    assert!(empty.to_string().contains("at least one row and one column"));
    let invalid = Error::InvalidCell { index: 1, rows: 1..2, cols: 1..3, grid: (2, 2) };
    assert!(invalid.to_string().contains("plot 1"));
    assert!(invalid.to_string().contains("1..3"));
    let overlap = Error::CellsOverlap { first: 1, second: 2 };
    assert!(overlap.to_string().contains("plots 1 and 2"));
}
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test --lib -- figure error::tests`
Expected: compile errors (no `figure` module and no new variants).

- [ ] **Step 3: Implement `src/figure.rs`, the error variants, and the exports**
  - **`GridSpan`:** the standard sealed-trait pattern, a `pub trait` in a private module, implemented for `usize`, `Range<usize>`, `RangeInclusive<usize>`, `RangeFrom<usize>`, `RangeTo<usize>`, `RangeToInclusive<usize>` and `RangeFull`. Resolve as in the spec: `i` → `i..i+1`, `a..=b` → `a..b+1`, `a..` → `a..len`, `..b` → `0..b`, `..=b` → `0..b+1`, `..` → `0..len`; every `+1` is `saturating_add(1)`. `.plot` resolves both spans right away, against `self.rows` and `self.cols`, and stores them; it checks nothing.
  - **`validate`, one pass:**
    - First, `rows == 0 || cols == 0` is `EmptyGrid`.
    - Then, for each plot `j` in order: a span with `start >= end`, or with `end` past the grid, is `InvalidCell { index: j, .. }`.
    - Otherwise, compare `j` with each earlier plot `i`, in order. Their rows overlap when `a.start < b.end && b.start < a.end`, and likewise for columns. Overlap in both is `CellsOverlap { first: i, second: j }`.
    - Pairwise only: nothing proportional to `rows * cols`.
  - **`slot_start`:**

    ```rust
    let (base, extra) = (length as usize / count, length as usize % count);
    (index * base + index.min(extra)) as u32 // index <= count, so this is at most `length`
    ```
  - **`cell_rect`:** columns from `slot_start(c.start, cols, width)` to `slot_start(c.end, cols, width)`, rows likewise with `height`.
  - **`Display` copy:**
    - `EmptyGrid`: "a figure needs at least one row and one column, but this one has {rows} rows and {cols} columns".
    - `InvalidCell`: "plot {index} in the figure covers rows {r.start}..{r.end} and columns {c.start}..{c.end}, which is empty or outside the figure's {grid.0} rows and {grid.1} columns".
    - `CellsOverlap`: "plots {first} and {second} in the figure cover the same slot".

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib -- figure error::tests`
Expected: PASS. Then run `cargo clippy --all-targets -- -D warnings`, which must be clean (the sealed traits must not trigger `private_bounds`).

- [ ] **Step 5: Commit**

```bash
git add src/figure.rs src/error.rs src/lib.rs src/prelude.rs
git commit -m "Add Figure: a grid of plots with spanning cells, and its validation"
```

### Task 4: Drawing a figure

**Files:**
- Modify: `src/figure.rs`
- Modify: `tests/golden.rs`; create `tests/snapshots/figure_grid.png`, `tests/snapshots/figure_spanning.png`
- Modify: `tests/properties.rs`

**Interfaces:**
- Consumes: `Plot::render_with` and `Defaults` (Task 2); `Figure::validate` and `cell_rect` (Task 3).
- Produces:

```rust
impl Figure {
    /// RGB8 pixels, `width * height * 3` bytes, top row first. Base text size: `font_size`, else
    /// `DEFAULT_FONT_SIZE`.
    pub fn render(&self, width: u32, height: u32) -> Result<Vec<u8>>;
    /// Size: `size`, else `DEFAULT_PNG_SIZE`.
    pub fn save_png(&self, path: impl AsRef<Path>) -> Result<()>;
    pub fn show(&self) -> Result<()>;
    pub fn show_in(&self, terminal: &Terminal) -> Result<()>;
    /// Renders with `font_size` as the base text size (Task 6's live frames use it).
    pub(crate) fn render_at(&self, width: u32, height: u32, font_size: u32) -> Result<Vec<u8>>;
    /// `size`, else `terminal.default_plot_size()`.
    pub(crate) fn size_in(&self, terminal: &Terminal) -> (u32, u32);
    /// `font_size`, else `terminal.text_size()`.
    pub(crate) fn font_size_in(&self, terminal: &Terminal) -> u32;
}
```

- [ ] **Step 1: Write the failing unit tests** (in `figure.rs`'s tests)

```rust
use crate::{plotting::colors, terminal::{Terminal, WindowSize}};

fn sine() -> Plot {
    Plot::new().line((0..50).map(|i| (i, (f64::from(i) / 5.0).sin())).collect::<Vec<_>>())
}

/// The `w` x `h` block at (`left`, `top`) of an RGB8 image `width` pixels wide.
fn block(rgb: &[u8], width: u32, (left, top, w, h): (u32, u32, u32, u32)) -> Vec<u8> {
    (top..top + h)
        .flat_map(|y| {
            let start = ((y * width + left) * 3) as usize;
            rgb[start..start + (w * 3) as usize].to_vec()
        })
        .collect()
}

#[test]
fn a_one_by_one_figure_is_its_plot() {
    let plot = sine().title("one");
    let fig = Figure::new(1, 1).plot(0, 0, plot.clone());
    assert!(fig.render(320, 240).unwrap() == plot.render(320, 240).unwrap());
    let fig = fig.background(colors::WHITE).font_size(20);
    let expected = plot.background(colors::WHITE).font_size(20);
    assert!(fig.render(320, 240).unwrap() == expected.render(320, 240).unwrap());
}

#[test]
fn each_plot_is_drawn_in_its_cell() {
    let left = sine().title("left");
    let right = sine().title("right").background(colors::WHITE);
    let fig = Figure::new(1, 3).plot(0, 0, left.clone()).plot(0, 1..3, right.clone());
    let rgb = fig.render(301, 200).unwrap(); // slots of 101, 100 and 100 pixels
    assert!(block(&rgb, 301, (0, 0, 101, 200)) == left.render(101, 200).unwrap());
    assert!(block(&rgb, 301, (101, 0, 200, 200)) == right.render(200, 200).unwrap());
}

#[test]
fn slots_without_a_plot_are_the_figure_background() {
    let rgb = Figure::new(2, 2)
        .plot(0, 0, sine())
        .background(colors::WHITE)
        .render(200, 100)
        .unwrap();
    assert!(block(&rgb, 200, (100, 0, 100, 100)).iter().all(|&b| b == 255));
    assert!(block(&rgb, 200, (0, 50, 200, 50)).iter().all(|&b| b == 255));
    let empty = Figure::new(2, 3).background(colors::WHITE).render(30, 20).unwrap();
    assert!(empty.len() == 30 * 20 * 3 && empty.iter().all(|&b| b == 255));
}

#[test]
fn a_plots_own_size_is_ignored_in_a_figure() {
    let fig = Figure::new(1, 1).plot(0, 0, sine().size(50, 50));
    assert!(fig.render(320, 240).unwrap() == sine().render(320, 240).unwrap());
}

#[test]
fn cells_without_pixels_fail_with_canvas_too_small() {
    let fig = Figure::new(1, 5).plot(0, 4, sine());
    assert!(matches!(fig.render(3, 100), Err(Error::CanvasTooSmall { .. })));
    // a huge grid has 1-pixel slots: an error, not a hang or an allocation per slot
    let huge = Figure::new(usize::MAX, usize::MAX).plot(0, 0, sine());
    assert!(matches!(huge.render(10, 10), Err(Error::CanvasTooSmall { .. })));
}

#[test]
fn an_invalid_grid_draws_nothing() {
    let fig = Figure::new(1, 1).plot(0, 1, sine());
    assert!(matches!(fig.render(100, 100), Err(Error::InvalidCell { .. })));
}

#[test]
fn show_matches_the_terminal_unless_set() {
    let terminal = Terminal::with_window(WindowSize {
        rows: 50, cols: 160, x_pix: 1600, y_pix: 1700, pix_per_row: 34, pix_per_col: 10,
    });
    let fig = Figure::new(1, 1);
    assert_eq!(fig.size_in(&terminal), terminal.default_plot_size());
    assert_eq!(fig.font_size_in(&terminal), 28);
    let fig = fig.size(300, 200).font_size(12);
    assert_eq!((fig.size_in(&terminal), fig.font_size_in(&terminal)), ((300, 200), 12));
}

#[test]
fn save_png_writes_a_readable_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("figure.png");
    Figure::new(1, 2).plot(0, 0, sine()).plot(0, 1, sine()).size(320, 240).save_png(&path).unwrap();
    let img = image::open(&path).unwrap();
    assert_eq!((img.width(), img.height()), (320, 240));
}
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test --lib figure`
Expected: compile errors (no `render`, `save_png`, `size_in` or `font_size_in`).

- [ ] **Step 3: Implement in `src/figure.rs`**
  - **`render_at`:**
    1. `self.validate()?`.
    2. Fill a `width * height * 3` buffer with `self.background`.
    3. For each plot `i`: `(left, top, w, h) = self.cell_rect(i, width, height)` and `rgb = plot.render_with(w, h, Defaults { background: self.background, font: &self.font, font_size })?`. Copy each of its `h` rows (`w * 3` bytes) into the figure row `top + y` at column `left` with `copy_from_slice`.
  - **`render`:** `render_at(width, height, self.font_size.unwrap_or(DEFAULT_FONT_SIZE))`.
  - **`save_png`:** like `Plot::save_png`, with `self.size.unwrap_or(DEFAULT_PNG_SIZE)`.
  - **`show_in`:** `render_at(size_in, font_size_in)`, then `terminal.show_rgb`. `show` is `Terminal::connect()?` then `show_in`.
  - **Doc example on `Figure`:** a runnable one that builds a 2×2 figure with a spanning bottom plot, calls `render(400, 300)` and asserts the length is `400 * 300 * 3`.

- [ ] **Step 4: Run the unit tests**

Run: `cargo test --lib figure`
Expected: PASS.

- [ ] **Step 5: Add the golden scenes** to `tests/golden.rs`

Split `check` into `check_rgb(name: &str, width: u32, height: u32, actual: Vec<u8>)`, which holds today's comparison body, and `check`, which calls it with `canvas.draw().expect("scene should draw").get_bytes()`. Import `termplt::{Figure, Plot}`.

```rust
fn samples(f: fn(f64) -> f64) -> Vec<(f64, f64)> {
    (0..=100).map(|i| f64::from(i) * 0.1).map(|x| (x, f(x))).collect()
}

#[test]
fn figure_grid() {
    // tick labels of different widths, titles on three plots, an x name on one: in this first
    // layout the plot areas do not line up
    let (w, h) = (800, 560);
    let fig = Figure::new(2, 2)
        .plot(0, 0, Plot::new().line(samples(|x| 20.0 + 3.0 * x.sin())).title("Temperature A").y_label("temp (°C)"))
        .plot(0, 1, Plot::new().line(samples(|x| 21.0 + 2.5 * (0.8 * x).cos())).title("Temperature B"))
        .plot(1, 0, Plot::new().line(samples(|x| 101300.0 + 600.0 * (0.5 * x).sin())).title("Pressure").y_label("Pa").x_label("time (s)"))
        .plot(1, 1, Plot::new().line(samples(|x| 101000.0 + 400.0 * (0.7 * x).cos())))
        .font_size(14);
    check_rgb("figure_grid", w, h, fig.render(w, h).expect("figure should draw"));
}

#[test]
fn figure_spanning() {
    let (w, h) = (640, 480);
    let wide = Plot::new()
        .line(Series::from(samples(f64::sin)).with_label("sin"))
        .line(Series::from(samples(f64::cos)).with_label("cos"))
        .title("Signals");
    let fig = Figure::new(2, 2)
        .plot(0, .., wide)
        .plot(1, 0, Plot::new().scatter(samples(|x| (x * 1.7).sin() * x)).title("Scatter"))
        .plot(1, 1, Plot::new().line_points(samples(|x| x * x)).title("Square"))
        .background(colors::WHITE);
    check_rgb("figure_spanning", w, h, fig.render(w, h).expect("figure should draw"));
}
```

Run `TERMPLT_UPDATE_SNAPSHOTS=1 cargo test --test golden figure_`, then `cargo test --test golden`. Expected: all pass. Open both PNGs:
- `figure_grid`: four plots, with the lower-left plot's left edge further right than the upper-left one's (the six-digit labels).
- `figure_spanning`: a wide plot with a legend on top, two plots below, and a white background with dark axes everywhere.

- [ ] **Step 6: Add the property tests** to `tests/properties.rs` (reuse its `series()` strategy; import `termplt::Figure`)

```rust
#[test]
fn figure_rendering_never_panics(
    rows in 0usize..4,
    cols in 0usize..4,
    cells in prop::collection::vec((series(), (0usize..5, 0usize..5), (0usize..5, 0usize..5)), 0..5),
    width in 0u32..400,
    height in 0u32..400,
    font_size in 1u32..40,
) {
    let mut fig = Figure::new(rows, cols).font_size(font_size);
    for (s, (r0, r1), (c0, c1)) in cells {
        fig = fig.plot(r0..r1, c0..c1, Plot::new().series(s));
    }
    // an error (an invalid grid, a cell too small) is fine; a panic or a short image is not
    if let Ok(rgb) = fig.render(width, height) {
        prop_assert_eq!(rgb.len(), width as usize * height as usize * 3);
    }
}

#[test]
fn a_one_by_one_figure_renders_like_its_plot(
    series in series(),
    width in 0u32..300,
    height in 0u32..300,
    font_size in 1u32..40,
) {
    let plot = Plot::new().series(series).font_size(font_size);
    let alone = plot.render(width, height);
    let in_figure = Figure::new(1, 1).plot(0, 0, plot).render(width, height);
    prop_assert_eq!(alone.is_ok(), in_figure.is_ok());
    if let (Ok(a), Ok(b)) = (alone, in_figure) {
        prop_assert!(a == b);
    }
}
```

Run: `cargo test --test properties`, then `cargo test --release --test properties`.
Expected: PASS both times. The debug run matters, because release builds don't check for integer overflow.

- [ ] **Step 7: Commit**

```bash
git add src/figure.rs tests
git commit -m "Draw figures: each plot rendered in its cell and copied into one image"
```

### Task 5: Docs and example for static figures

**Files:**
- Create: `examples/subplots.rs`, `docs/images/subplots.png`
- Modify: `README.md`, `scripts/readme_images.sh`, `CLAUDE.md`, `CHANGELOG.md`

**Interfaces:**
- Consumes: the public `Figure` API (Tasks 3–4).

- [ ] **Step 1: Write `examples/subplots.rs`**

It builds one figure: a wide plot across the top row (two labeled series, so it has a legend) and two plots below it, with titles. With a path argument, it calls `.size(800, 600).save_png(path)`; with none, it calls `.show()`. Open with a module doc comment in the style of `examples/live.rs`, including the command (`cargo run --example subplots`).

- [ ] **Step 2: Generate the image and look at it**

Run: `cargo run --release --example subplots -- docs/images/subplots.png`
Expected: an 800×600 PNG with three plots, the top one spanning the width. Open it and check.

- [ ] **Step 3: README, script, CLAUDE.md, CHANGELOG**
  - **README feature bullet:** add to "Features": "**Subplots** — `Figure::new(2, 2).plot(1, .., plot)` draws several plots as one image on a grid; a plot can span rows or columns".
  - **README section:** a "### Subplots" section between "### Live plots" and "### Full control", with:
    - a `rust,no_run` example that builds the same figure as `examples/subplots.rs`, with its data inline, and calls `fig.show()?`, ending with `# Ok::<(), termplt::Error>(())` like the other README examples;
    - one sentence each on the defaults (the figure's `background`, `font` and `font_size` apply to plots without their own) and on what isn't there yet (plot areas don't line up across cells when their labels differ);
    - the image, in the same `<img … src="https://raw.githubusercontent.com/EdCarney/termplt/main/docs/images/subplots.png" />` form as the others.
  - **`scripts/readme_images.sh`:** before the final `echo`, add `cargo run --release --quiet --example subplots -- "$out/subplots.png"`.
  - **`CLAUDE.md`:**
    - In "Public API layers", a `Figure` (`figure.rs`) bullet: the grid, `GridSpan`, the settings precedence, the same output methods as `Plot`.
    - A short "### Figures (`figure.rs`)" subsection under Architecture with the five drawing steps from the spec's "Drawing a figure".
    - The new golden scenes and property tests under Testing.
  - **`CHANGELOG.md`:** under `## [Unreleased]`, an `### Added` entry for `Figure` (grid, spanning cells via `GridSpan`, `render`/`save_png`/`show`/`show_in`, figure-wide background, font and text size), and for `Error::EmptyGrid`, `Error::InvalidCell` and `Error::CellsOverlap`.

- [ ] **Step 4: Milestone checks**

Run the six commands from "Working on this plan". `cargo test` runs the README example as a doctest.
Expected: all pass.

- [ ] **Step 5: Commit, push, PR**

```bash
git add examples/subplots.rs docs/images/subplots.png README.md scripts/readme_images.sh CLAUDE.md CHANGELOG.md
git commit -m "Document figures and add the subplots example"
git push -u origin multi-plot-figure
```

Open a PR against `multi-plot-empty-axes`, titled "Add Figure: several plots in one image". The description:
- says it implements the spec's "Public API" and "Drawing a figure" sections (static part);
- says it is stacked on PR 1;
- shows the two new golden images.

---

## Milestone 3: Live figures

Branch `multi-plot-live` from `multi-plot-figure`.

### Task 6: `LivePlot` draws plots and figures

**Files:**
- Modify: `src/plot.rs`: `Render`, the sealed `Draw` trait, generic `update`, shared live start, tests
- Modify: `src/figure.rs`: `impl Render for Figure`, `show_live`, `show_live_in`, `show_live_with`
- Modify: `src/lib.rs`: `pub use plot::{DEFAULT_PNG_SIZE, LivePlot, Plot, Render};`
- Create: `examples/subplots_live.rs`
- Modify: `README.md`, `CLAUDE.md`, `CHANGELOG.md`

**Interfaces:**
- Consumes: `Figure::render_at`, `size_in` and `font_size_in` (Task 4).
- Produces:

```rust
// plot.rs
/// What a [`LivePlot`] can draw: a [`Plot`] or a [`Figure`]. Sealed.
pub trait Render: sealed::Draw {}
pub(crate) mod sealed {
    pub trait Draw {
        /// RGB8 pixels at `width` x `height` with `font_size` as the base text size.
        fn render_rgb(&self, width: u32, height: u32, font_size: u32) -> crate::Result<Vec<u8>>;
    }
}
impl LivePlot {
    pub fn update(&mut self, drawing: &impl Render) -> Result<()>;
}
/// Draws the first frame of `drawing` and places it with `place`.
pub(crate) fn start_live(
    drawing: &impl Render,
    (width, height): (u32, u32),
    font_size: u32,
    terminal: &Terminal,
    place: impl FnOnce(&Terminal, &Image) -> Result<Placement>,
) -> Result<LivePlot>;

// figure.rs
impl Figure {
    pub fn show_live(&self) -> Result<LivePlot>;
    pub fn show_live_in(&self, terminal: &Terminal) -> Result<LivePlot>;
    pub(crate) fn show_live_with(
        &self,
        terminal: &Terminal,
        place: impl FnOnce(&Terminal, &Image) -> Result<Placement>,
    ) -> Result<LivePlot>;
}
```

- **For `Plot`,** `render_rgb` is today's `LivePlot::update` body, `canvas_with_font_size(width, height, font_size).draw()?.into_bytes()`. It ignores the plot's own `font_size`, as frames do today.
- **For `Figure`,** it is `render_at(width, height, font_size)`, so plots with their own `font_size` keep it.

- [ ] **Step 1: Write the failing tests** (in `plot.rs`'s tests, which can see `LivePlot`'s private fields; import `crate::Figure` and `sealed::Draw`)

```rust
#[test]
fn a_live_figure_is_redrawn_in_place() {
    let terminal = live_terminal();
    let empty = || Plot::new().line(Series::from(Vec::<(f64, f64)>::new()).with_label("v"));
    let mut fig = Figure::new(2, 1).plot(0, 0, empty()).plot(1, 0, empty()); // panels start empty
    let mut live = fig.show_live_with(&terminal, Terminal::place_in_buffer).unwrap();
    assert_eq!(live.size(), terminal.default_plot_size());
    assert_eq!(live.font_size, terminal.text_size());
    for t in 1..=3 {
        for plot in fig.plots_mut() {
            plot.series_mut()[0].push(t, t * t);
        }
        live.update(&fig).unwrap();
    }
    assert_eq!(count(live.placement.written(), b"a=t,"), 4);
    assert_eq!(count(live.placement.written(), b"a=d,"), 3);
}

#[test]
fn a_figure_frame_that_cannot_be_drawn_sends_nothing() {
    let terminal = live_terminal();
    let fig = Figure::new(1, 2).plot(0, 0, Plot::new().line(vec![(0, 0), (1, 1)]));
    let mut live = fig.show_live_with(&terminal, Terminal::place_in_buffer).unwrap();
    let written = live.placement.written().len();
    let overlapping = fig.clone().plot(0, .., Plot::new());
    assert!(matches!(
        live.update(&overlapping),
        Err(crate::Error::CellsOverlap { first: 0, second: 1 })
    ));
    assert_eq!(live.placement.written().len(), written);
    live.update(&fig).unwrap();
    assert_eq!(count(live.placement.written(), b"a=d,"), 1);
}

#[test]
fn one_live_plot_can_show_plots_and_figures_of_any_shape() {
    let terminal = live_terminal();
    let plot = Plot::new().line(vec![(0, 0), (1, 1)]);
    let mut live = plot.show_live_with(&terminal, Terminal::place_in_buffer).unwrap();
    live.update(&Figure::new(1, 2).plot(0, 0, plot.clone()).plot(0, 1, plot.clone())).unwrap();
    live.update(&Figure::new(3, 1).plot(1.., 0, plot.clone())).unwrap();
    live.update(&plot).unwrap();
    assert_eq!(count(live.placement.written(), b"a=t,"), 4);
    assert_eq!(live.size(), terminal.default_plot_size());
}

#[test]
fn a_live_figure_keeps_its_first_size_and_text_size() {
    let fig = Figure::new(1, 1)
        .plot(0, 0, Plot::new().line(vec![(0, 0), (1, 1)]))
        .size(320, 240)
        .font_size(11);
    let mut live = fig.show_live_with(&live_terminal(), Terminal::place_in_buffer).unwrap();
    assert_eq!((live.size(), live.font_size), ((320, 240), 11));
    live.update(&fig.clone().size(100, 100).font_size(30)).unwrap();
    assert_eq!((live.size(), live.font_size), ((320, 240), 11));
}

#[test]
fn frames_use_the_base_text_size_except_where_a_figures_plot_has_its_own() {
    let own = Plot::new().line(vec![(0, 0), (1, 1)]).font_size(30);
    // a plot drawn alone takes the frame's text size, as in 0.4.0
    assert!(own.render_rgb(320, 240, 20).unwrap() == own.clone().font_size(20).render(320, 240).unwrap());
    // a plot in a figure keeps its own
    let fig = Figure::new(1, 1).plot(0, 0, own.clone()).font_size(11);
    assert!(fig.render_rgb(320, 240, 20).unwrap() == own.render(320, 240).unwrap());
    let plain = Plot::new().line(vec![(0, 0), (1, 1)]);
    let fig = Figure::new(1, 1).plot(0, 0, plain.clone()).font_size(11);
    assert!(fig.render_rgb(320, 240, 20).unwrap() == plain.font_size(20).render(320, 240).unwrap());
}
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test --lib plot::tests`
Expected: compile errors (no `Render`/`Draw`, `update` takes `&Plot`, no `Figure::show_live_with`).

- [ ] **Step 3: Implement**
  - **`plot.rs`:** add the `sealed` module, `Render` and `impl Render for Plot`. Make `LivePlot::update` generic: `drawing.render_rgb(width, height, self.font_size)`, then `self.placement.replace_rgb(..)`. Move `Plot::show_live_with`'s body into the free `start_live` above. `Plot::show_live_with` then calls it with `(self.size.unwrap_or_else(|| terminal.default_plot_size()), self.font_size_in(terminal))`.
  - **`figure.rs`:** `impl sealed::Draw for Figure` and `impl Render for Figure`. `Figure::show_live_with` calls `crate::plot::start_live` with `(self.size_in(terminal), self.font_size_in(terminal))`. `show_live_in` passes `Terminal::place`, and `show_live` connects first.
  - **Docs:** `LivePlot::update`'s doc now says a size or text size set on the plot or figure since the first frame is ignored, and text sizes set on a figure's plots still apply. Copy the `show_live*` docs from `Plot`'s, adapted.

- [ ] **Step 4: Run the library tests**

Run: `cargo test --lib`
Expected: PASS. The existing live tests, such as `a_live_plot_is_redrawn_in_place_at_its_first_size` and `live_plots_are_send_and_sync`, still pass unchanged.

- [ ] **Step 5: Example and docs**
  - **`examples/subplots_live.rs`:** two panels stacked (`Figure::new(2, 1)`), both starting empty.
    - The top panel is a noisy signal; the bottom is its running mean.
    - 200 samples, 50 ms apart, keeping the last 100 (`keep_last`).
    - Make the noise deterministic, for example from a sum of sines, since there are no new dependencies.
    - Header comment in the style of `examples/live.rs`, with `cargo run --example subplots_live`.
  - **README:**
    - In "### Subplots", add two lines on live figures: `let mut live = fig.show_live()?;` and `live.update(&fig)?` after changing `fig.plots_mut()`, plus the example command.
    - Extend the Features bullet with "static or live".
  - **`CLAUDE.md`:** `LivePlot::update(&impl Render)` takes a plot or a figure (sealed `Render`), in the public API layers.
  - **`CHANGELOG.md`:** under `## [Unreleased]`, `### Added`:
    - `Figure::show_live`/`show_live_in`;
    - `LivePlot::update` taking a plot or a figure through the sealed `Render` trait;
    - the note that a reference that only coerced to `&Plot` (such as `&Box<Plot>`) now needs `&*`.

- [ ] **Step 6: Milestone checks**

Run the six commands from "Working on this plan", and `cargo build --examples`.
Expected: all pass. If you have a Kitty-protocol terminal, run `cargo run --example subplots_live` and watch both panels update in place. Otherwise say in the PR that this was not checked by eye.

- [ ] **Step 7: Commit, push, PR**

```bash
git add src examples/subplots_live.rs README.md CLAUDE.md CHANGELOG.md
git commit -m "Draw figures live: LivePlot::update takes a plot or a figure"
git push -u origin multi-plot-live
```

Open a PR against `multi-plot-figure`, titled "Live figures". The description:
- says it implements the spec's live-figures parts (`Render`, `Figure::show_live`);
- says it is stacked on PR 2;
- includes a manual checklist: Kitty, Ghostty, WezTerm, tmux, with `examples/subplots_live.rs`.

Tell the user all three PRs are open, their order, and anything left unchecked.
