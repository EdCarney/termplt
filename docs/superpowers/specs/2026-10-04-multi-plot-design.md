# Subplots: a grid of plots in one image

**Spec for 0.5.0** · 2026-10-04 · Status: design approved in brainstorming; written spec under review

## Goal

Draw several plots as one image, arranged on a grid the way matplotlib's `subplots` and
`GridSpec` arrange axes. A figure can be rendered, saved as a PNG, shown in the terminal, or
shown live and redrawn in place as its data changes. This round is library only: the CLI does
not change.

**Success criteria**

1. `Figure::new(2, 2).plot(0, 0, a).plot(0, 1, b).plot(1, .., c)` draws plots `a` and `b` side
   by side on top and `c` across the whole bottom row, in one image.
2. `Figure` has the output methods `Plot` has (`render`, `save_png`, `show`, `show_in`,
   `show_live`, `show_live_in`) with the same defaults.
3. `let mut live = fig.show_live()?;` then `live.update(&fig)?` after changing the plots' data
   redraws the whole grid in place, through the existing `Placement` path, with no protocol
   changes. `live.update(&plot)` still compiles and behaves exactly as in 0.4.0.
4. A 1×1 figure renders byte for byte the same image as its plot rendered alone at the same
   size, text size, background and font.
5. A plot with no data draws empty axes (0 to 1, or its explicit limits), alone or in a figure,
   instead of failing with `Error::NoData`.
6. Every existing golden image still matches. No new dependencies; the MSRV stays 1.88.
7. The aligned layout (see "Next: aligned layout" below) can be added later without changing
   the public API.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Scope | Library only; static and live figures | The CLI (a `--layout`/`panel=` design) can come later on top of the library API. |
| How cells are laid out (v1) | **Independent**: each cell's plot is rendered on its own at the cell's size, then copied into the figure | No change to `canvas.rs` layout. Plot areas in a grid column or row do not line up when their tick labels, titles or axis names differ; that is fixed next (below). |
| Next layout | **Aligned**, like matplotlib's `layout="constrained"`: every cell in a grid column gets the column's widest left margin, every cell in a row the row's tallest top and bottom margins | Chosen as the follow-up. It becomes the default when it lands. |
| A layout option | None in v1 | matplotlib's grids always line up their plot areas, so aligned is the expected default. With no option in v1, the aligned layout is a rendering improvement (golden images change), not an API change or a behaviour flip behind an option. An opt-out can be added later if anyone needs it. |
| API shape | `Figure::new(rows, cols)`, then `.plot(rows, cols, plot)` where `rows`/`cols` are one index or a range (`1`, `0..2`, `0..=1`, `1..`, `..2`, `..`) | One method covers single cells and spanning cells, in the by-value builder style of `Plot`. Like matplotlib's `gs[1, :]`. Rejected: a mosaic string (`"AB\nCC"`, needs parsing, errors only at draw time) and pre-made cells like `plt.subplots` (clumsy with by-value builders, spanning becomes a separate step). |
| Spanning cells | In v1 | A wide plot over two small ones is a common layout. |
| Unequal row heights / column widths | Not in v1 | matplotlib's `height_ratios`/`width_ratios`. Cells are stored with row and column ranges, so ratios only change the slot arithmetic later. |
| The cell type | The existing `Plot` | Every `Plot` builder (series, limits, title, labels, legend, grid) works in a cell; no parallel "axes" type. |
| Figure versus plot settings | The figure sets `size`, `background`, `font` and `font_size`. A plot uses its own `background`, `font` and `font_size` where one was set on it, the figure's otherwise. A `size` set on a plot in a figure is ignored | Like matplotlib's figure-wide defaults with per-axes overrides. `Plot` already stores `font_size` as an `Option`; `background` and `font` become `Option`s privately so "not set" differs from "set to the default". |
| Invalid grids | Reported when drawing, as `InvalidLimits` is; the builders stay infallible and chainable | Zero rows or columns, a range that is empty or outside the grid, and two plots covering one slot. |
| Plots with no data | Empty axes everywhere, alone and in figures: 0 to 1 on each axis without explicit limits, as matplotlib draws empty axes | Live panels can start empty instead of failing the whole frame. A behaviour change for code that matched `Err(NoData)`; recorded in the CHANGELOG. |
| Live figures | `LivePlot` handles plots and figures: `update(&mut self, drawing: &impl Render)`, with `Render` a sealed trait implemented by `Plot` and `Figure` | One live type; existing `update(&plot)` calls still compile. `Render` rather than `Frame`, because "frame" already means one live image and the legend box. |
| A cell's drawing error | Passed up unchanged (for example `CanvasTooSmall` for a cell too small) | Wrapping it with the cell's index can come later if it is needed. |
| Rendering order | Cells one after another | Parallel rendering would need a new dependency. |

## Architecture

```
Figure::render(w, h)
  ├── validate the grid                         # Error::EmptyGrid / InvalidCell / CellsOverlap
  ├── resolve each plot's settings              # its own background, font, font_size, else the figure's
  ├── cell rectangles                           # slot widths w / cols, remainder to the first columns;
  │                                             #   a spanning cell is the union of its slots
  ├── each cell: Plot canvas at the cell size → draw()      (unchanged single-plot pipeline)
  └── copy each cell's rows into the figure buffer           (slots without a plot keep the figure background)

Figure::show_live() ──► LivePlot { placement, font_size }
LivePlot::update(&impl Render)   render at the first frame's size and text size → placement.replace_rgb()
```

| Unit | Responsibility | Depends on |
|---|---|---|
| `plotting/graph.rs` | Empty axes: `view_limits` and `scale` work without visible data | none |
| `plotting/canvas.rs` | The marker and line inset is 0 when the graph has no series | none |
| `error.rs` | `Error::EmptyGrid`, `Error::InvalidCell`, `Error::CellsOverlap`; docs of `NoData` and `NoVisibleData` | none |
| `plot.rs` | `background` and `font` stored as `Option`s; a crate-private way to render a plot with figure defaults; the `Render` trait; `LivePlot::update` generic over it | `graph.rs`, `canvas.rs` |
| `figure.rs` (new) | `Figure`, the `GridSpan` trait, grid validation, cell rectangles, combining the cells, output methods | `plot.rs`, `error.rs` |
| `lib.rs`, `prelude.rs` | Export `Figure`, `GridSpan`, `Render`; `Figure` in the prelude | `figure.rs`, `plot.rs` |

## Public API

All additive except the empty-axes behaviour change. Version 0.5.0.

```rust
/// Several plots drawn as one image, on a grid of equal-size slots. A plot can cover one slot
/// or a block of them.
#[derive(Debug, Clone, PartialEq)]
pub struct Figure { /* rows, cols, plots: Vec<Plot>, spans: Vec<(Range<usize>, Range<usize>)>,
                       size, background, font, font_size */ }

impl Figure {
    /// A figure with `rows` x `cols` slots and no plots yet.
    pub fn new(rows: usize, cols: usize) -> Figure;

    /// Adds `plot` covering `rows` and `cols`: one index or a range of each, counted from the
    /// top-left slot. `..` means every row or column.
    pub fn plot(self, rows: impl GridSpan, cols: impl GridSpan, plot: Plot) -> Self;

    /// The image size in pixels. By default `show` fits the terminal and `save_png` uses
    /// `DEFAULT_PNG_SIZE`, as for `Plot`.
    pub fn size(self, width: u32, height: u32) -> Self;
    /// The background of slots without a plot, and of every plot without its own background.
    pub fn background(self, color: RGB8) -> Self;
    /// The font of every plot without its own font.
    pub fn font(self, font: Font) -> Self;
    /// The base text size of every plot without its own. By default `show` matches the
    /// terminal's text, and other output uses `DEFAULT_FONT_SIZE`, as for `Plot`.
    pub fn font_size(self, px: u32) -> Self;

    /// The plots, in the order they were added.
    pub fn plots(&self) -> &[Plot];
    /// The plots, in the order they were added, for changing their data in place (live figures).
    pub fn plots_mut(&mut self) -> &mut [Plot];

    /// Draws the figure and returns its RGB8 pixels (`width * height * 3` bytes, row-major, top
    /// row first).
    pub fn render(&self, width: u32, height: u32) -> Result<Vec<u8>>;
    pub fn save_png(&self, path: impl AsRef<Path>) -> Result<()>;
    pub fn show(&self) -> Result<()>;
    pub fn show_in(&self, terminal: &Terminal) -> Result<()>;
    pub fn show_live(&self) -> Result<LivePlot>;
    pub fn show_live_in(&self, terminal: &Terminal) -> Result<LivePlot>;
}

/// Rows or columns a plot covers in a [`Figure`]: one index (`1`) or a range (`0..2`, `0..=1`,
/// `1..`, `..2`, `..`). Implemented for `usize` and the standard range types; sealed.
pub trait GridSpan: private::Sealed {}

/// What a [`LivePlot`] can draw: a [`Plot`] or a [`Figure`]. Sealed.
pub trait Render: private::Sealed {}

impl LivePlot {
    /// Draws `drawing` over the previous frame, at the size and base text size of the first
    /// frame (a size or text size set on the plot or figure since then is ignored; text sizes
    /// set on a figure's plots still apply).
    pub fn update(&mut self, drawing: &impl Render) -> Result<()>;
}

pub enum Error {
    // ...existing variants...
    /// A figure has no rows or no columns.
    EmptyGrid { rows: usize, cols: usize },
    /// A plot in a figure covers no slot, or slots outside the grid.
    InvalidCell {
        /// The plot's position in `Figure::plots`.
        index: usize,
        /// The rows and columns it covers, resolved to ranges (`1` is `1..2`, `..` is `0..len`).
        rows: Range<usize>,
        cols: Range<usize>,
        /// The figure's rows and columns.
        grid: (usize, usize),
    },
    /// Two plots in a figure cover the same slot.
    CellsOverlap {
        /// Positions in `Figure::plots`, the earlier first.
        first: usize,
        second: usize,
    },
}
```

**Exports:** `pub use figure::{Figure, GridSpan};` and `Render` next to `LivePlot` in `lib.rs`
(`pub use plot::{DEFAULT_PNG_SIZE, LivePlot, Plot, Render};`); `Figure` in the prelude.
`GridSpan` and `Render` only appear in bounds, so callers never need to import them.

**`GridSpan` resolution.** `.plot` resolves each span against the figure's rows or columns
right away, since `Figure::new` fixed them, and stores a `Range<usize>`: `i` becomes `i..i+1`,
`a..=b` becomes `a..b+1` (saturating), `a..` becomes `a..len`, `..b` becomes `0..b`, `..`
becomes `0..len`. Nothing is checked at that point; an empty or out-of-grid range is
`InvalidCell` when drawing. The sealed supertrait holds one crate-private method, for example
`fn resolve(self, len: usize) -> Range<usize>`.

**`Render`.** The sealed supertrait holds one crate-private method that renders RGB8 at a given
size with a given base text size, for example
`fn render_rgb(&self, width: u32, height: u32, font_size: u32) -> Result<Vec<u8>>`. For a
`Plot` this is exactly today's `LivePlot::update` body
(`canvas_with_font_size(width, height, font_size).draw()?.into_bytes()`), so plot frames are
unchanged. For a `Figure`, `font_size` is the figure-level base: plots with their own
`font_size` keep it. The method name must differ from the inherent `render` methods. Making
`update` generic keeps `live.update(&plot)` compiling, but a reference that only coerced to
`&Plot` (such as `&Box<Plot>`) now needs `&*`; record that in the CHANGELOG.

**`Plot` changes (private).** `background: Option<RGB8>` (`None` means black, the current
default) and `font: Option<Font>` (`None` means the built-in font). `Plot::graph()`,
`Plot::canvas()`, `render`, `save_png` and `show*` behave exactly as before. A crate-private
method renders a plot with figure defaults: the background, font and base text size to use
where the plot has none. The foreground and grid colors in `Plot::graph()` must come from the
background actually used, so a plot on a figure's white background gets dark axes. A side
effect worth knowing: `Plot::new().background(BLACK) != Plot::new()` now, because "set to
black" and "not set" differ.

## Drawing a figure

1. **Validate.** `rows == 0 || cols == 0` is `EmptyGrid`. For each plot, in order: a range with
   `start >= end`, or `end` beyond the grid, is `InvalidCell`. Then mark slots: a slot already
   covered by an earlier plot is `CellsOverlap { first: earlier, second: this }`. A figure with
   no plots is valid and draws as its background.
2. **Slots.** For width `w` and `cols` columns: `base = w / cols`, `extra = w % cols`; column
   `c` is `base + 1` pixels wide when `c < extra`, else `base`, and starts at
   `c * base + min(c, extra)`. Rows are the same, counted from the top. The slots cover the
   figure exactly: no gap at the right or bottom edge, and the figure is exactly the requested
   size. A plot covering rows `r0..r1` and columns `c0..c1` gets the rectangle from the start of
   slot `(r0, c0)` to the end of slot `(r1 - 1, c1 - 1)`. A cell can be 0 pixels wide or high
   when the figure has more columns than pixels; drawing it then fails with the canvas's own
   `CanvasTooSmall`, like any plot too small for its text.
3. **Render each cell** with today's single-plot path at the cell's size:
   `canvas_with_font_size(cell_w, cell_h, font_size)` with the resolved background and font,
   then `draw()`. The edge buffer (`(min(w, h) / 40).max(8)` in `Plot::canvas_with_font_size`)
   is computed from the cell's size, so it also spaces neighbouring cells. Any error is returned
   as is.
4. **Combine.** Start from a buffer filled with the figure background. Both buffers store rows
   top first, so each cell row is one `copy_from_slice` into the figure row at the cell's
   offset.
5. **Output.** `render` returns the buffer. `save_png` uses `size` or `DEFAULT_PNG_SIZE`.
   `show_in` uses `size` or `terminal.default_plot_size()`, and the base text size
   `font_size` or `terminal.text_size()`, then `terminal.show_rgb`. `show_live_in` places the
   first frame with `Terminal::place` exactly as `Plot::show_live_in` does, and the returned
   `LivePlot` keeps that size and base text size. Share the plot's live-placement code (today
   `Plot::show_live_with`, which takes the placing function so tests can use
   `Terminal::place_in_buffer`) between the two rather than copying it.

## Empty axes

A separate change that lands first. It also applies to a `Plot` drawn alone.

- **View limits.** `Graph::view_limits()` still validates explicit limits first
  (`InvalidLimits`). Only one case is new: when no point is visible (no finite points at all,
  or none inside the explicit limits), each axis with explicit limits uses them as today.
  An axis without explicit limits takes the range of all the finite data, with the usual
  margin and padding, as matplotlib scales each axis from all the data (so data with y from 1
  to 9 and `x_limits(100, 200)` keeps a y axis of about 0.6 to 9.4, and a live plot with a
  fixed x window does not jump to 0 to 1 when the window is briefly empty). Only without any
  finite data is such an axis 0 to 1, with no margin or padding, as matplotlib draws empty
  axes. Whenever some point is visible, the view is computed exactly as today. The
  private `Graph::visible()` must not fail on a graph without finite points: there is nothing
  to clip.
- **Scaling.** `Graph::scale` and the canvas's `scale_with_view` return a graph whose series are
  empty instead of failing with `NoVisibleData`.
- **Layout.** In `TerminalCanvas::layout_with`, the largest marker or line size is 0 when the
  graph has no series (today `.max().ok_or(Error::NoData)`). Series that exist but are empty
  still count, so their styles set the inset as today.
- **Legend.** Labeled empty series keep their entries, as in matplotlib. With no points,
  `LegendLocation::Best` scores every location 0 and takes the first (upper right), which is
  also what matplotlib's `best` picks.
- **Unchanged.** `Graph::limits()` (the data extent) and `Drawable::get_mask` for `Graph` (which
  works in pixel coordinates without a view) still fail with `NoData` on a graph without finite
  points. The tests `limits_with_no_data_returns_error` and
  `get_mask_on_empty_graph_returns_error` in `graph.rs` stay. A `TerminalCanvas` without a graph
  draws as today.
- **Errors.** The `NoData` and `NoVisibleData` variants stay (removing a variant breaks code
  matching on it). Their docs change: `NoData` is now returned only by `Graph::limits` and
  `Graph::get_mask`; drawing a plot doesn't need data. `NoVisibleData` is no longer returned
  and is kept so code matching it still compiles.
- **Live and CLI.** `Plot::show_live` on an empty plot now shows empty axes, and
  `LivePlot::update` with emptied series redraws empty axes. The CLI filters non-finite
  points and reports "no data points found" itself before building a plot, so `--follow` still
  waits for its first point. Static output with `--xlim`/`--ylim` that exclude every point no
  longer fails: it draws empty axes and prints `warning: no data points lie within the axis
  limits (--xlim/--ylim); drawing empty axes` to stderr (exit status 0; never in `--follow`).
- **Tests that change.**
  - `canvas.rs` `empty_graph_returns_error` now expects empty axes.
  - `graph.rs` `scale_with_limits_excluding_all_points_returns_error` now expects empty
    series and the view at the explicit limits.
  - `plot.rs` `empty_plot_is_an_error` now expects an image with axes.
  - `plot.rs` `a_frame_that_cannot_be_drawn_sends_nothing` needs another failing frame, for
    example inverted limits (`x_limits(1.0, 0.0)`, which is `InvalidLimits`), where it used an
    empty plot.

## Testing

**Unit tests (`figure.rs`)**
- Slots: remainders go to the first columns and rows; the slots of a 3×3 grid at 100×100 cover
  exactly 100 pixels each way; a spanning cell is the union of its slots.
- `GridSpan`: every implementing type resolves as listed above, including `..=usize::MAX`
  without overflow.
- Each error: `EmptyGrid` for 0 rows and for 0 columns; `InvalidCell` for an empty range and
  for a range past the grid, with the plot's index; `CellsOverlap` naming both plots, the
  earlier first.
- Settings: a plot's own background, font and text size win; otherwise the figure's apply. A
  plot without a background on a white figure gets dark axes.
- A 1×1 figure is byte for byte its plot rendered alone (same size, text size, background and
  font). A slot without a plot is the figure background. A figure without plots is all
  background.

**Live (`plot.rs` tests, with `Terminal::place_in_buffer`)**
- A figure's first frame has the terminal's default size and text size, and each update sends
  one transmission and one delete.
- A frame that can't be drawn (an invalid grid) sends nothing, and the next good frame replaces
  the last one shown.
- One `LivePlot` updated with a plot and then with a figure works.
- A live figure whose panels start empty draws (needs the empty-axes change).

**Empty axes** (`graph.rs`, `canvas.rs`): the view is 0 to 1 without data; explicit limits on
one axis with no data give those limits on it and 0 to 1 on the other; points all outside the
explicit limits give the limits and no error, with an axis without limits keeping the data's
range; the legend of an empty labeled series is drawn upper right.

**Golden images** (`tests/golden.rs`, new snapshots, reviewed before committing): a 2×2 grid
with different tick label widths and titles; a spanning layout (one wide plot over two); an
empty plot with a title and a labeled series. Every existing snapshot must match unchanged.

**Property tests** (`tests/properties.rs`): random grids (1–4 rows and columns), random spans,
including invalid ones, random sizes and random data never panic; a successful render has
`width * height * 3` bytes; a 1×1 figure equals its plot.

**No pty test**: live figures go through the same `Placement` path, already tested byte for
byte.

## Docs

- **README:** a "Subplots" section after the live plots section: a `rust,no_run` example with a
  spanning cell and `save_png`, its image `docs/images/subplots.png`, and two lines on live
  figures (`fig.show_live()`, `live.update(&fig)`). README is the crate doc, so the example is a
  doctest.
- **Examples:** `examples/subplots.rs` builds the README figure and saves it to the path given
  as its argument (used by `scripts/readme_images.sh`) or shows it when no path is given;
  `examples/subplots_live.rs` streams two panels (a signal and its running mean) into a live
  figure (`cargo run --example subplots_live`).
- **`scripts/readme_images.sh`:** add
  `cargo run --release --quiet --example subplots -- "$out/subplots.png"`.
- **CLAUDE.md:** `Figure` in the public API layers, the figure drawing steps, empty axes under
  the rendering pipeline, the new tests.
- **CHANGELOG.md**, under a new `[Unreleased]` section (0.5.0): Added (`Figure`, `GridSpan`,
  `Render`, `Error::EmptyGrid`, `Error::InvalidCell`, `Error::CellsOverlap`, `LivePlot::update`
  accepting figures); Changed (a plot without data draws empty axes instead of failing with
  `NoData`; `NoVisibleData` is no longer returned).

## Delivery

Three PRs against `main`, each green on its own (tests, clippy, fmt, docs, MSRV).

| PR | Content | Needs |
|---|---|---|
| 1. Empty axes | The "Empty axes" section: `graph.rs`, `canvas.rs`, error docs, the changed tests, the empty-plot golden image, CHANGELOG | none |
| 2. Static figures | `Figure`, `GridSpan`, the three errors, `Plot`'s optional `background`/`font` and figure-default rendering, validation, slots, combining, `render`/`save_png`/`show`/`show_in`, unit, golden and property tests, `examples/subplots.rs`, README section and image, CLAUDE.md, CHANGELOG | none (can run alongside PR 1) |
| 3. Live figures | `Render`, `LivePlot::update(&impl Render)`, `Figure::show_live`/`show_live_in` sharing the plot's placement code, live tests, `examples/subplots_live.rs`, README live lines, CLAUDE.md, CHANGELOG | PRs 1 and 2 |

0.5.0 is tagged after PR 3, when the user asks.

## Next: aligned layout

Not part of this spec; recorded so the follow-up starts from what is known. It goes between
steps 2 and 3 of "Drawing a figure"; the public API stays the same.

- **Idea.** Measure each cell's plot-area margins (left, top, bottom; the right margin is the
  same for every cell), take the largest left margin in each grid column and the largest top and
  bottom margins in each grid row, then lay every cell out with at least those margins. A new
  plot height can change the y tick labels and so the left margin, so repeat until nothing
  changes, as `TerminalCanvas::layout` already repeats for stacked text.
- **Evidence.** A throwaway mockup during brainstorming simulated it with today's API: read each
  cell's plot area with `TerminalCanvas::get_drawable_limits()`, then re-render the narrower
  cells with extra per-side buffer (`BufferType::TopBottomLeftRight`). On a 2×2 grid
  with labels from `17` to `102000`, titles on three cells and an x-axis name on one, the
  margins settled in two passes. The plot areas were 36 px apart in the left column and 23 and
  25 px apart at the top and bottom of the lower row before aligning, and matched exactly after.
- **What it needs.** A crate-private hook in `canvas.rs` to measure margins and to lay out with
  minimum margins, instead of abusing the buffer. Spanning cells take part in every grid column
  and row they cover. Cells' own text sizes may differ; alignment only compares margins.
- **Effect.** Existing figure golden images change; plot-only images don't.

## Out of scope

- The CLI (`--layout`, a `panel=` key in `--series`, several panels in `--follow`).
- The aligned layout (next), shared axes (`sharex`/`sharey`) and hiding inner tick labels
  (matplotlib's `label_outer`).
- Unequal row heights or column widths; spacing options (`wspace`/`hspace`).
- A figure title (`suptitle`) and a figure-wide legend.
- A mosaic-string constructor (`Figure::mosaic("AB\nCC")`).
- Rendering cells in parallel; reusing unchanged cells between live frames.
- Wrapping a cell's drawing error with the cell's index.
- A default figure size that depends on the number of rows (the default is `Plot`'s).
