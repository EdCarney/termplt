# Legend and series names

**Spec 2 of 2 for 0.3.0** · 2026-09-26 · Status: approved in brainstorming, awaiting review of this document

Spec 1 (`2026-09-25-text-rendering-design.md`) added TrueType text, titles, axis names and CLI names from headers. This spec adds the legend. 0.3.0 is released when both are done.

## Goal

Finish IMPROVEMENTS.md item 13: series get names, and a plot with named series shows a legend. The library matches matplotlib's default legend (`loc="best"`, framed, 80% opaque). The CLI names series on its own and shows the legend whenever there are two or more series.

**Success criteria**

1. `Plot::new().line(Series::from(data).with_label("sin"))` draws a legend. So does a hand-built `Graph` drawn on a `TerminalCanvas`.
2. `termplt a.csv b.csv` shows a legend named from the headers, or from the file names when the headers don't tell the series apart. A single series shows no legend unless `--legend` is given.
3. With `LegendLocation::Best` the legend goes where matplotlib's `loc="best"` would put it: the scoring and the order of the candidates are ported rule for rule.
4. The legend never covers more than half the plot's width, and never runs past the plot. Long names wrap onto 2 lines and then end with `…`. Series that don't fit are counted in a `+N more` row.
5. Plots without names are unchanged: every existing golden image still matches.
6. Output stays byte-identical on Linux, macOS and Windows: golden images compare exactly.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| When the legend shows (library) | Whenever any series has a label; `with_legend(false)` / `legend(false)` hides it | Naming a series is the request. Existing plots have no labels, so they don't change. |
| When the legend shows (CLI) | With 2 or more series; `--legend`, `--no-legend` and `--legend-loc` override | One series needs no key. The CLI names series on its own, so the library rule alone would show a legend on every plot. |
| Naming API | `Series::with_label` only | The user's call. `Plot::line(Series::from(..).with_label(..))` keeps the label because `line` only sets styles. "Label" is matplotlib's word and the CLI's `label=` key. |
| CLI names | `label=`, then the y column header, then the file stem (with `: column` for files that give several series); inline data has no name | A plain `termplt a.csv b.csv` gets a useful legend. |
| Placement | Inside the plot. `Best` (default) picks the emptiest of matplotlib's ten positions; the ten fixed positions can be chosen | matplotlib's default. The plot keeps its full size. |
| Look | matplotlib's legend defaults: sizes in em, rounded frame in the background color at 80% opacity, light edge | Plots look familiar. Chosen from rendered mockups. |
| Overflow | At most half the plot wide and the plot tall; labels wrap to 2 lines then `…`; a `+N more` row for the rest | Terminal plots are small. Same wrap-then-ellipsis rule as titles in spec 1. |
| Architecture | `Graph` holds the settings, `TerminalCanvas` draws the legend, `plotting/legend.rs` computes it | Every way of drawing gets legends. `draw()` already holds the scaled series that "best" needs. |

**Deliberate differences from matplotlib**
- **Big markers and thick lines:** an entry is tall enough for its sample, and the sample column widens for a marker wider than 2 em, or a line too thick to show 1 em of straight line within 2 em. matplotlib lets them spill into the next row; pixel markers can be large next to 14 px text.
- **Overflow:** the size caps, wrapping, `…` and the `+N more` row. matplotlib's legend grows without limit.
- **Edge:** always 1 px, colored from the text and background colors (on black text over white this is matplotlib's `0.8` gray exactly). matplotlib's edge is 1 pt, so it scales with dpi.
- **Blending:** the 80% frame is blended in linear light with `Canvas::blend`, like text. matplotlib's Agg blends in sRGB, so data under the frame shows through slightly differently.
- **Hidden labels:** only a missing or empty label keeps a series out of the legend. matplotlib also hides labels that start with `_`.

## Architecture

```
TerminalCanvas::draw()
  ├── layout()                     # unchanged: the legend takes no space outside the plot
  ├── grid, axes, series           # unchanged
  ├── legend::build(..)            # labeled series → entries: measure, wrap, cap, "+N more"
  ├── legend::place(..)            # the fixed location, or "best": score 10 candidates
  │                                #   against the scaled series
  ├── frame → Canvas::blend        # rounded, anti-aliased, 80% opaque
  ├── samples → draw_segment / draw_marker
  └── text                         # legend labels join the other PlacedText, drawn last
```

| Unit | Responsibility | Depends on |
|---|---|---|
| `plotting/legend.rs` (new) | Public `LegendLocation`. Crate-private: build the entries and the box size under the caps, compute the ten candidate positions, score them, and rasterize the frame's coverage. Pure functions of sizes, text measurements and points, so each is unit-tested on its own. | `font.rs` (measuring), `text::wrap`, `limits.rs`, `series.rs` |
| `plotting/series.rs` | The `label` field and its builder and getter; `map_points` carries it | none new |
| `plotting/graph.rs` | The legend settings, their builders and getters; `clone_without_data` copies them | `legend.rs` for `LegendLocation` |
| `plotting/canvas.rs` | Calls `legend.rs` after the series are drawn, blends the frame, draws the samples with the existing marker and line stamps, and adds the labels to the text list | `legend.rs`, `line.rs`, `marker.rs` |
| `plot.rs` | `legend` and `legend_location` builders, passed on in `Plot::graph()` | `graph.rs` |
| CLI `names.rs` (new) | The naming rules as one pure function | none |
| CLI `cli.rs`, `series.rs`, `main.rs` | The flags, the `label=` key, wiring, `--verbose` lines | `names.rs` |

## Public API

All additive; nothing breaks.

```rust
// plotting::legend, also in the prelude
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LegendLocation {
    #[default]
    Best,
    UpperRight,
    UpperLeft,
    LowerLeft,
    LowerRight,
    Right,          // the same position as CenterRight, as in matplotlib
    CenterLeft,
    CenterRight,
    LowerCenter,
    UpperCenter,
    Center,
}

impl Series {
    /// Names the series in the legend. An empty label is the same as none.
    pub fn with_label(self, text: impl Into<String>) -> Self;
    /// The label; `None` when unset or empty.
    pub fn label(&self) -> Option<&str>;
}

impl Graph {
    /// Whether a legend is drawn when some series has a label. Default: true.
    pub fn with_legend(self, show: bool) -> Self;
    /// Where the legend goes. Default: `LegendLocation::Best`.
    pub fn with_legend_location(self, location: LegendLocation) -> Self;
    pub fn legend_visible(&self) -> bool;
    pub fn legend_location(&self) -> LegendLocation;
}

impl Plot {
    pub fn legend(self, show: bool) -> Self;
    pub fn legend_location(self, location: LegendLocation) -> Self;
}
```

The variants are in the order of matplotlib's `loc` codes 0 to 10; the "best" search tries them in that order. `TerminalCanvas` gets no new API.

```rust
Plot::new()
    .line(Series::from(sin).with_label("sin"))
    .scatter(Series::from(samples).with_label("samples"))
    .legend_location(LegendLocation::UpperLeft)   // optional
    .show()?;
```

## Legend layout

**Entries:** the series with a label, in the order they were added. Nothing is drawn when there are none, or when the legend is hidden.

**Text:** the tick labels' size and color (`label_style()`), and the canvas's font. matplotlib's legend and tick labels are both "medium". A label is wrapped with `text::wrap(label, text_width, 2, measure)`, so `\n` starts a new line and text that doesn't fit ends with `…`. Lines are 1.2 em apart (`LINE_SPACING`, as in spec 1).

**Sizes:** matplotlib's defaults, in em of the text size, turned into whole pixels as spec 1's layout does: `(em × size).round()`.

| Constant | Value | matplotlib |
|---|---|---|
| `BORDER_PAD` | 0.4 em, inside the frame on every side | `legend.borderpad` |
| `LABEL_SPACING` | 0.5 em between entries | `legend.labelspacing` |
| `HANDLE_LENGTH` | 2.0 em, the sample width | `legend.handlelength` |
| `HANDLE_TEXT_PAD` | 0.8 em between the sample and the text | `legend.handletextpad` |
| `BORDER_AXES_PAD` | 0.5 em between the frame and the plot edges | `legend.borderaxespad` |
| `CORNER_RADIUS` | 0.2 em | `fancybox`: `round,rounding_size=0.2` |
| `FRAME_ALPHA` | 204 of 255 (0.8) | `legend.framealpha` |
| `MAX_LABEL_LINES` | 2 | none |

**Samples:**
- A series with a line gets a horizontal line across the sample in its own `LineStyle`: color, thickness, and for a dashed line the 6-on/4-off pattern starting at the sample's left edge.
- A series with a marker gets one marker, at its own size, in the middle of the sample (matplotlib's `numpoints=1`, `markerscale=1`). Line and marker together give both.
- A series with neither gets an empty sample and its label.
- The sample column is `max(HANDLE_LENGTH, widest marker diameter, widest line's 2 × thickness + 1 plus 1 em)` wide, the same for every entry, so the labels line up. The extra em keeps a thick line's sample a line between its round ends, not a disc.

**One entry:** the sample's vertical center sits on the text's center line: for one line, the middle of the digit height (as y tick labels are placed in spec 1); for two lines, halfway between the two lines' centers (matplotlib's `multilinebaseline`). The entry is as tall as the text block and the sample together (a marker is `2 × size + 1` px tall, a line `2 × thickness + 1`).

**The box:** `BORDER_PAD`, the sample column, `HANDLE_TEXT_PAD` and the widest text line across; `BORDER_PAD`, the entries with `LABEL_SPACING` between them, and `BORDER_PAD` down. The frame's edge is inside the box.

**Caps and overflow:**
1. The box is at most `plot width / 2` wide (rounded down). The text width left for labels is that cap minus the pads and the sample column; labels wrap to it.
2. The box is at most `plot height − 2 × BORDER_AXES_PAD` tall.
3. If every entry fits, all are shown. Otherwise the legend keeps the first `k` entries, as many as fit together with a last row `+N more` (N = the entries left out). That row is one text line in the text column, with no sample, `LABEL_SPACING` below the last entry.
4. No legend is drawn when not even one entry fits with that row, or when the text width is narrower than `…`.

## Placement

**Candidates:** a box of the legend's size, `BORDER_AXES_PAD` in from the edges of the plot area (`Layout::plot`, the area the data is mapped onto), at each fixed location. A centered box that can't sit exactly in the middle moves half a pixel toward the lower left. `Right` and `CenterRight` give the same box.

**A fixed location** uses its own box. **`Best`** scores the ten fixed locations in enum order, a port of matplotlib's `Legend._find_best_position`:
- Every series counts, labeled or not, using the points `draw()` has already scaled to canvas pixels (clipped to the limits, with NaN gaps).
- **A series with a line** (matplotlib's `Line2D`) scores 1 for each finite point strictly inside the box, plus 1 if its path touches the box: a segment crosses it or lies inside it. Segments with a non-finite end are not drawn, so they are skipped.
- **A series with markers and no line** (matplotlib's scatter offsets) scores 1 for each finite point strictly inside the box.
- A series with neither scores nothing. Marker size is ignored, as in matplotlib.
- The first location that scores 0 wins at once. Otherwise the lowest score wins, and a tie goes to the earlier location.

The path test is a standard segment-against-rectangle clip test on `f64`, and a line stops testing at its first touching segment. The cost is at most ten passes over the points.

## Drawing

After the series and before any text:

1. **Frame:** a rounded rectangle filling the box, anti-aliased from its signed distance. The fill is the canvas background; a 1 px edge just inside the boundary uses the edge color: `0.8 × background + 0.2 × text color` per channel in sRGB, rounded (`#cccccc` for black text on white, `#333333` for white on black). Both are blended with `Canvas::blend` at `FRAME_ALPHA` times their coverage, so the data underneath shows through at 20%.
2. **Samples:** drawn solid with the existing stamps (`draw_segment` with a `LineStamp`, `draw_marker` with `marker_stamp`), which keeps them identical to the series in the plot.
3. **Labels and `+N more`:** `PlacedText` in the shared text list, drawn with the rest of the text.

## CLI

**Series names**, computed by one pure function in `names.rs` after all series are loaded. Each series brings its `label=` value if any, its file path as written (`-` for stdin, none for inline data), its y column's header name, and its y column's 1-based index.

1. **`label=`** always wins and is never changed. `label=` with nothing after it means no name. `label` joins the `--series` keys; values run until a `,` or `;` followed by `key=`, so names can contain commas.
2. **Inline data** (`--data`, `--series data=…`) without `label=` has no name.
3. **A file series** is named by its y column's header. Without a header it takes the file stem (`stdin` for `-`, and the path as written when the path has no stem); when the same file gives several series, it takes `stem: index` (`data: 2`).
4. **Collisions:** a name from rule 3 that equals another series' name is replaced with `stem` for a file that gives one series, or `stem: column` for a file that gives several (the header name, or the index without a header). Where that still collides because two files share a stem, the path as written replaces the stem.
5. Names that still repeat are kept; colors and markers tell those series apart.

| Command | Names |
|---|---|
| `termplt a.csv b.csv`, both with a `value` header | `a`, `b` |
| `termplt d.csv -y temp -y rh` | `temp`, `rh` |
| `termplt d.txt -y 2 -y 3`, no header | `d: 2`, `d: 3` |
| `termplt run1.csv run2.csv -y temp -y rh` | `run1: temp`, `run1: rh`, `run2: temp`, `run2: rh` |
| `termplt runs/1/d.csv runs/2/d.csv`, both with a `value` header | `runs/1/d.csv`, `runs/2/d.csv` |
| `termplt --data "(1,2),(3,4)" d.csv` with a `value` header | `value` only |

**Flags** (help heading "Plot"):

| Flag | Effect |
|---|---|
| `--legend` | Show the legend, even for one series |
| `--no-legend` | Hide it. With `--legend`, the last of the two wins (clap `overrides_with`). |
| `--legend-loc <LOC>` | `best`, `upper-right`, `upper-left`, `lower-left`, `lower-right`, `right`, `center-left`, `center-right`, `lower-center`, `upper-center`, `center`. A clap `ValueEnum` mapped to `LegendLocation`; completions list the values. Implies `--legend`. |

The legend is shown when `--no-legend` is not in effect and either `--legend` or `--legend-loc` is given or there are 2 or more series. The library still draws it only when some series has a name.

**Verbose:** the series line gains `label="sin"` or `label=none`, and one new line reads `[verbose] legend: on, best` or `[verbose] legend: off`.

## Testing

**Unit tests**
- **`legend.rs`, size:** the box size at 14 px from the constants, for one entry, two entries, and a two-line entry.
- **`legend.rs`, caps:** a long label wraps at the half-width cap and ends with `…` after 2 lines; too many entries give `+N more` with the right N; nothing is built when one entry can't fit or the text width is narrower than `…`.
- **`legend.rs`, candidates:** exact boxes for all ten locations with the 0.5 em pad; `Right` equals `CenterRight`.
- **`legend.rs`, scoring:**
  - points strictly inside count, and points on the edge don't;
  - a line crossing the box with no point inside scores 1;
  - a line lying inside the box scores its points plus 1;
  - marker-only series score points and get no path term;
  - segments at a NaN gap are skipped;
  - the first zero wins; a tie goes to the earlier location.
- **`legend.rs`, colors:** the edge color for black on white, white on black, and a colored background.
- **`Series` and `Graph`:** labels round-trip; an empty label reads as `None`; `map_points` keeps the label; `Graph` defaults to visible and `Best`; `clone_without_data` copies both settings.
- **Canvas:**
  - a labeled graph with `with_legend(false)` draws the same bytes as the same graph without labels;
  - `get_drawable_limits()` is the same with and without labels;
  - a label's text pixels are drawn over the frame;
  - a fixed location puts the frame's corner pixel where the candidate box says.

**Golden images** (`tests/golden.rs`): the 10 existing scenes are unchanged. New scenes:
- `legend_best`: three series, "best" picks a free corner;
- `legend_fixed_location`: upper left, with line, dashed and marker samples;
- `legend_overflow`: a small canvas, long labels that wrap, and `+N more`;
- `legend_light_background`: the frame colors on white.

**Property tests** (`tests/properties.rs`): with arbitrary labels (empty, whitespace, very long, any Unicode), series counts, styles, canvas sizes and font sizes, drawing never panics, and every pixel that differs between the render with labels and the render without them lies inside `get_drawable_limits()`.

**CLI**
- `names.rs`: one test per rule and per row of the examples table, plus `label=` with no value and a repeated column.
- `series.rs`: the `label=` key, including a name with a comma.
- `cli.rs`: `--legend` and `--no-legend`, last one wins; `--legend-loc` values.
- `main.rs`: `--legend-loc` implies the legend and `--no-legend` overrides it; the verbose lines.
- `tests/cli.rs`: with two files, the PNG differs with and without `--no-legend`.

## Docs

- **README:** a library example with `with_label`; a "Legend" part of the CLI section with an image from `scripts/readme_images.sh`; rows for the three flags and the `label=` key.
- **CLAUDE.md:** the pipeline diagram and a legend paragraph under Text Rendering; the CLI section's naming rules; remove "No legend yet" from the known issues.
- **IMPROVEMENTS.md:** item 13 done; its stale body is replaced.
- **CHANGELOG.md**, under `[0.3.0] - Unreleased`: Added (`Series::with_label`, `LegendLocation`, the `Graph` and `Plot` legend settings, CLI names, `--legend`, `--no-legend`, `--legend-loc`, `label=`); Changed: CLI plots with 2 or more series now show a legend (`--no-legend` restores the old look).

## Delivery

Two PRs to `main`, each green on its own (tests, clippy, fmt, docs, MSRV). The second is opened only after the first is merged, against `main`, so neither merges into a stale base. No new dependencies; the MSRV stays 1.88.

1. **Library legend:** `Series` labels, `LegendLocation`, the `Graph` and `Plot` settings, `plotting/legend.rs`, the canvas drawing, golden and property tests, the README library example, and the CHANGELOG's library entries.
2. **CLI and docs:** `names.rs`, the flags and the `label=` key, `--verbose`, CLI tests, the README CLI section and image, CLAUDE.md, IMPROVEMENTS.md, and the rest of the CHANGELOG.

0.3.0 is tagged after PR 2, when the user asks.

## Out of scope

- A legend title, several columns (`ncols`), and a legend outside the plot (`bbox_to_anchor`).
- Custom entries or handles, and hiding a named series from the legend.
- A separate legend text size or color, a shadow, and samples after the text (`markerfirst=False`).
- Anti-aliased samples (the series themselves are not anti-aliased).
- The deferred minors from spec 1's review, other than the stale IMPROVEMENTS.md item 13 text.
