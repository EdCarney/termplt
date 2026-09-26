# Text rendering: TrueType font, titles and axis names

**Spec 1 of 2 for 0.3.0** · 2026-09-25 · Status: approved in brainstorming, awaiting review of this document

Spec 2, the legend and series names, follows this one. 0.3.0 is released when both are done.

## Goal

Add plot titles and axis names to termplt, the first part of IMPROVEMENTS.md item 13. The bitmap font only draws `0-9 . - + e`, so this needs a real font. Replace it with an embedded TrueType font, and draw all text with it, tick labels included.

**Success criteria**

1. `Plot::new().title(..).x_label(..).y_label(..)` and `termplt --title/--xlabel/--ylabel` draw a title and axis names.
2. Titles and axis names that are too long wrap onto more lines: up to 3 for a title and 2 for an axis name. Text that still doesn't fit ends with `…`.
3. `termplt temps.csv` with a header row names its axes from the header automatically when that's unambiguous.
4. In the terminal, text matches the terminal's own text size, including on HiDPI screens. In PNGs it matches matplotlib's default size.
5. Latin-1, Greek and common math symbols (`µ`, `°`, `Δ`, `σ`, `≤`, `−`, `²`) work out of the box. Users can supply their own font for other scripts.
6. Output is byte-identical on Linux, macOS and Windows: golden images compare exactly.
7. No text overlaps other text at canvas sizes from 300×200 to 800×600.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Font technology | TrueType, rasterized with anti-aliasing | Any size and smooth edges. The user's call: "if we can use TTF we should". |
| Rasterizer | `ab_glyph` 0.2, `default-features = false, features = ["std"]` | Pure Rust, about 4 small crates, one code path on every platform. `fontdue`'s default x86 SIMD path could make pixels differ between CI runners. |
| Font | Go Regular 2.010, trimmed to the characters in `assets/fonts/charset.txt` | Chosen from rendered samples of DejaVu Sans, Go, Inter and JetBrains Mono. BSD-3-Clause, about 25 KB. |
| Character coverage | Latin-1, Greek and common math symbols, plus user fonts via `Plot::font` / `--font` | Covers units and symbols in plot labels. Users bring their own font for anything else. |
| Default size | Match the terminal's text in `show()`; matplotlib's default (10 pt at 100 dpi, about 14 px) in PNGs | Readable on any display. PNGs don't depend on the environment, so CI and local output match. |
| Automatic axis names (CLI) | From CSV headers when every series agrees | A plain `termplt file.csv` gets labeled axes. |
| Drawing approach | Text gets its own draw path: coverage bitmaps blended into the canvas | The mask path only has solid pixels. Series already bypass it in the same way. |
| Behavior reference | matplotlib's defaults, unless there's a stated reason to differ | Plots look familiar. The differences are listed below. |

**Deliberate differences from matplotlib**
- **Collisions:** a title that would overlap the y offset, and an x offset that would overlap the x axis name, move to their own line. matplotlib does this only for the title.
- **Long text:** titles and axis names wrap and end with an ellipsis. matplotlib lets them overflow unless `wrap=True`.
- **Offsets:** these are 0.2.1 behavior, kept: offsets come from the view range and are printed with all their digits.

## Architecture

```
Plot / CLI ─► TerminalCanvas { font, base size } ─► layout(): bands sized from measured text
                                                   ─► draw: grid, axes, series (solid pixels, as today)
                                                        └► text: lay out and rasterize each label
                                                             ─► coverage bitmap (turned 90° for the y name)
                                                             ─► Canvas::blend in linear light
```

| Unit | Responsibility | Depends on |
|---|---|---|
| `plotting/font.rs` (new, public `Font`) | Parse fonts. Look up glyphs, falling back to Go. Lay out one line: advances, plus kerning where the font has it. Measure it. Rasterize it to an 8-bit coverage bitmap and rotate that bitmap by 90°. | `ab_glyph` |
| `plotting/text.rs` (reworked) | `TextStyle`, `TextPositioning`, `Label`. A pure `wrap()`. | `font.rs` for measuring |
| `Canvas::blend` (in `canvas.rs`) | Mix a color into a pixel by coverage, in linear light, using integer tables | a committed sRGB table |
| `canvas.rs` layout | Band functions: title, y offset, plot, tick labels, x name, x offset. Tick fitting uses measured text. | `font.rs`, `text.rs`, `ticks.rs` |
| `plotting/ticks.rs` | Unchanged, except numbers use the Unicode minus | none |
| `plotting/numbers.rs` | **Deleted**: the bitmap glyphs and their tests | none |

Series, markers, lines, axes and grid still draw solid pixels as they do today. `Series::draw_into` and its equivalence test with `get_mask` are untouched.

## Public API

### Added

```rust
// plotting::font, also in the prelude
pub struct Font { .. }                        // Clone (cheap, Arc), Debug, PartialEq, Send + Sync
impl Default for Font                         // the embedded Go font
impl Font {
    /// A TrueType or OpenType font (the first face of a collection). Characters it doesn't
    /// have are drawn with the built-in Go font.
    pub fn from_bytes(data: impl Into<Vec<u8>>) -> Result<Font>;
}

impl Plot {
    pub fn title(self, text: impl Into<String>) -> Self;
    pub fn x_label(self, text: impl Into<String>) -> Self;
    pub fn y_label(self, text: impl Into<String>) -> Self;
    pub fn font(self, font: Font) -> Self;
    /// Base text size in pixels. Default: the terminal's text size in `show()`,
    /// DEFAULT_FONT_SIZE otherwise.
    pub fn font_size(self, px: u32) -> Self;
}

impl Graph {
    pub fn with_title(self, text: impl Into<String>) -> Self;
    pub fn with_x_label(self, text: impl Into<String>) -> Self;
    pub fn with_y_label(self, text: impl Into<String>) -> Self;
    pub fn title(&self) -> Option<&str>;
    pub fn x_label(&self) -> Option<&str>;
    pub fn y_label(&self) -> Option<&str>;
}

impl TerminalCanvas {
    pub fn with_font(self, font: Font) -> Self;
    pub fn with_font_size(self, px: u32) -> Self;  // clamped to 1..=MAX_FONT_SIZE
}

impl Terminal {
    /// The base text size (pixels) that matches the terminal's own text: the row height / 1.2.
    pub fn text_size(&self) -> u32;
}

pub const DEFAULT_FONT_SIZE: u32 = 14;          // in plotting::text: matplotlib's 10 pt at 100 dpi
pub const MAX_FONT_SIZE: u32 = 400;             // in plotting::text
Error::InvalidFont(String)                      // Error is #[non_exhaustive], so this is additive
```

### Changed (breaking)

| 0.2 | 0.3 |
|---|---|
| `TextStyle::new(color, scale, padding)` | `TextStyle::new(color, size_px)`, with the size clamped to `1..=MAX_FONT_SIZE`. `TextStyle::with_color(color)` is unchanged and means "the default size for this text's role". |
| `TextStyle::scale()`, `padding()` | `TextStyle::size() -> Option<u32>` |
| `MAX_TEXT_SCALE`, `MAX_TEXT_PADDING` | `MAX_FONT_SIZE` |
| `Text` (`new`, `from_number`, `width`, `height`) | Removed. Measuring needs a font, so it's internal now. |
| `Label::new(Text, TextPositioning)`, `txt()`, `limits()` | `Label::new(text: impl Into<String>, style: TextStyle, pos: TextPositioning)`, `text()`, `style()`, `pos()` |
| Rendered images | New text everywhere |

These keep their signatures: `TextPositioning`, `TerminalCanvas::with_label`, `Axes::new(positioning, TextStyle)`. `TextStyle` still derives `Copy`, `Eq` and `Default` (black, no size). Sizes are whole pixels, so `Eq` stays on `TextStyle` and `Axes`.

### Which size applies

- **Base size.** Set by `TerminalCanvas::with_font_size`, `DEFAULT_FONT_SIZE` otherwise.
  - `Plot::canvas` and `save_png` use `Plot::font_size`, or `DEFAULT_FONT_SIZE`.
  - `Plot::show_in` uses `Plot::font_size`, or `Terminal::text_size()`: `round(pix_per_row / 1.2)`, clamped to `8..=MAX_FONT_SIZE`, since a terminal line is about 1.2 em. For example, 34 px rows give 28 px text.
- **Roles.** The axes' `TextStyle` covers tick labels, offset labels and axis names: its size if set, otherwise the base size. The title uses the same color at `round(1.2 × that size)`, matplotlib's `large`. A user `Label` with no size uses the base size.
- **What the size means.** A size is the em size in pixels, like CSS `font-size`. For `ab_glyph` it's converted as `PxScale = size × height_unscaled / units_per_em`.
- **Color.** Text color comes from the style. The existing rule still applies: a text color equal to the background becomes black or white.

## Text rendering

- **Glyph lookup.** A glyph comes from the canvas's font. If that font lacks the character, it comes from the built-in Go. If neither has it, the canvas font's `.notdef` glyph is used, usually a box. A line can mix glyphs from both fonts, on a shared baseline, at the same em size.
- **Layout.** Each glyph advances by its font's advance width, plus `ab_glyph` kerning when both neighbours come from a font with kerning data. Go has no `kern` or `GPOS` table. Control characters other than `\n` are drawn as spaces.
- **Rasterizing.** `ab_glyph` outlines are drawn into an 8-bit coverage bitmap for the whole line. The y-axis name's bitmap is rotated 90° counter-clockwise, so it reads bottom to top.
- **Blending.**
  - `Canvas::blend(x, y, color, coverage)` works in linear light with integer math only.
  - A committed `SRGB_TO_LINEAR: [u16; 256]` table decodes background and text to 16-bit linear, and they're mixed by coverage.
  - The result is encoded back by binary search on the same table, choosing the nearest sRGB value.
  - There are no runtime `powf` calls, so every platform produces the same bytes. A unit test checks the table against the sRGB formula to within ±1.
  - Pixels outside the canvas are skipped.
- **Drawing order.** Unchanged: grid, axes, series, then all text last.

## Layout

Placement follows matplotlib. Gaps are fractions of the base size, taken from matplotlib's padding at 10 pt:

| Gap | Size | matplotlib |
|---|---|---|
| tick labels to plot | 0.35 em | `xtick.major.pad` 3.5 pt |
| axis name to tick labels | 0.4 em | `axes.labelpad` 4 pt |
| title to plot | 0.6 em | `axes.titlepad` 6 pt |
| offset label to plot, or to tick labels | 0.3 em | `OFFSETTEXTPAD` 3 pt |

The buffer around the canvas edge (`BufferType`) stays as it is. The fixed `LABEL_GAP = 4` px is replaced by the tick gap.

**Bands**, from the outside in:
- **Top:** buffer, then the title block, then the y offset line.
  - The title's bottom line shares the y offset's line unless they collide (see below).
  - With no title, the y offset line stands alone, as in 0.2.1.
- **Bottom:** buffer, then the x name block, then the x tick labels.
  - The x offset shares the x name's top line unless they collide.
  - With no x name, the x offset has its own line below the tick labels, as in 0.2.1.
- **Left:** buffer, then the y name block, then the y tick labels.
- **Right:** buffer.
- **Sizing:**
  - A text line's height is the font's ascent minus its descent.
  - Lines within a wrapped block are 1.2 em apart (matplotlib's `linespacing`).
  - A rotated block is as wide as its line height times its line count. Its first line is farthest from the plot, the natural order for text that reads bottom to top.
  - Absent text adds no band, and neither does an empty or whitespace-only string.

**Placement**
- The title and x name are centered horizontally on the plot area, and the y name vertically. Each line of a wrapped block is centered.
- Text that would leave the canvas is shifted inward.
- x tick labels are centered under their tick.
- y tick labels are right-aligned to the tick-label column. They're centered on their tick by the height of the digit `0` in their font, not their whole line box (matplotlib's `center_baseline`).
- Offsets are placed as in 0.2.1: the y offset left-aligned at the y axis above the plot, and the x offset right-aligned at the plot's right edge below the tick labels.

**Collisions (repeated passes)**
1. Lay out with the title sharing the y offset's line, and the x offset sharing the x name's line.
2. A pair collides when its two boxes on the shared line would be less than 0.5 em apart horizontally. If a pair collides, lay out again with that pair stacked: the title goes a line above the y offset, or the x offset a line below the x name.

Stacking shrinks the plot, which re-wraps the y name and moves the x name and the y offset, so it can make the other pair collide. Layout repeats until no new pair collides. Stacked pairs stay stacked, so this ends within three passes. (The first version of this spec claimed a second pass could never collide; the final review showed otherwise.)

**Wrapping.** `wrap(text, max_width, max_lines, measure) -> Vec<String>` is a pure function with a measuring closure.
- It breaks at spaces and at `\n`.
- The maximum width is the canvas width inside the buffer. For the y name it's the canvas height. Using the canvas, not the plot area, avoids a loop: the plot area depends on the line counts.
- A title gets at most 3 lines and an axis name at most 2.
- Overflow is shown on the last line: trailing characters are removed until the text plus `…` fits.
- A single word wider than the maximum width is cut the same way, never broken mid-word.
- If not even `…` fits, the text is left out.
- Tick and offset labels never wrap.

**Small canvases.** Nothing is dropped to make room. If the bands leave no plot area, `draw` returns `CanvasTooSmall` as today, and the CLI's message suggests `--width`/`--height`.

**Numbers.** All generated numeric text uses the Unicode minus `−` (U+2212), as matplotlib does by default. That covers tick labels, scientific exponents (`2.5e−6`) and offsets (`−1e15`).

## CLI

A new **Text** help heading:

| Flag | Meaning |
|---|---|
| `--title <TEXT>` | Plot title |
| `--xlabel <TEXT>` | x-axis name. The default comes from the CSV header when all series agree; `""` gives none. Aliases: `--x-label`, `--x_label`. |
| `--ylabel <TEXT>` | Same for the y axis |
| `--font <FILE>` | A `.ttf`/`.otf` font, or the first face of a `.ttc`. Characters it lacks use Go. It gets a file-path completion hint. |
| `--font-size <PX>` | Base text size, `1..=MAX_FONT_SIZE` (clap range check). Default: the terminal's text size, or `DEFAULT_FONT_SIZE` with `--output`. |

**Automatic axis names**
1. `Table::points` also returns the header text of the x and y columns it resolved: `x_name`, `y_name`.
2. A column has no name when:
   - it's the row number (a single-column file, or `-x index`);
   - the table has no header;
   - the header cell is empty;
   - or the data came from `--data` or `--series data=…`.
3. `axis_names(&[SeriesNames]) -> (Option<String>, Option<String>)` names an axis only when every series has a name for it and all the names are identical. The comparison is exact: `Time` and `time` differ.
4. `--xlabel`/`--ylabel` override the automatic name, and `""` clears it.

Stdin with a header behaves like a file. `--series file=…` contributes its file's headers.

**Errors**
- `cannot read font 'x.ttf': <os error>`
- `'x.ttf' is not a TrueType or OpenType font: <reason>`, from `Error::InvalidFont`

**`--verbose`** adds one line, for example `text: 28 px (terminal rows are 34 px), font: Go (built in)` or `text: 14 px (default for --output), font: fonts/Inter.ttf`.

`--series` gets no new keys. A per-series `label=` belongs to spec 2.

## Font file and licensing

| Path | Contents |
|---|---|
| `assets/fonts/go-regular-subset.ttf` | The embedded font, about 25 KB (about 15 KB compressed in the crate), loaded with `include_bytes!` |
| `assets/fonts/charset.txt` | The character ranges, one per line, with comments. See the list below. |
| `assets/fonts/LICENSE-Go` | The font's license verbatim: BSD-3-Clause, © 2016 Bigelow & Holmes Inc. |
| `scripts/subset_font.sh` | Downloads the pinned source, checks its hash, and runs `pyftsubset` from a pinned `fonttools` in a temporary virtualenv. See the command below. |
| `tests/fixtures/digits-only.ttf` | A Go subset with only digits, made by the same script, to test fallback |

- **Characters in `charset.txt`:**
  - Latin-1: U+0020–007E and U+00A0–00FF.
  - Greek: U+0391–03A9 and U+03B1–03C9.
  - Math: `− ≤ ≥ ≈ ≠ ∞ √ ∆ ∑`.
  - Superscripts U+2070 and U+2074–2079, and subscripts U+2080–2089.
  - Arrows `← ↑ → ↓`.
  - Punctuation and symbols: `– — ‘ ’ “ ” • … ‰ €`.
- **Source:** `golang.org/x/image`, `font/gofont/ttfs/Go-Regular.ttf` at commit `41969df76e82aeec85fa3821b1e24955ea993001` (version 2.010), SHA-256 `197d9f3703b4c00af609178876a8d73e396f64fe438b2c871778566632374be3`.
- **Subset command:** `pyftsubset --unicodes-file=assets/fonts/charset.txt --no-hinting --name-IDs='*'`. `--name-IDs='*'` keeps the copyright and license records (name IDs 0 and 13) inside the font. Hinting is dropped because `ab_glyph` ignores it.
- **One character list:** `charset.txt` drives both the script and a Rust test that the embedded font has a glyph for every listed character, so they can't drift apart.
- **`Cargo.toml`:** `license = "MIT AND BSD-3-Clause"`.
- **Release archives:** the release workflow adds `LICENSE` and `LICENSE-Go` to every archive. BSD-3 requires the notice with binary distributions. This also fixes an existing gap: the 0.2.x archives hold only the binary, without termplt's MIT notice.
- **README:** the License section notes the bundled BSD-3-Clause Go font and links to `LICENSE-Go`.
- **Size:** the PR reports the release archive sizes before and after.

## Testing

**Unit tests**
- **`font.rs`:**
  - The embedded font covers `charset.txt`.
  - `from_bytes(b"not a font")` returns `InvalidFont`.
  - With the digits-only fixture, `A` falls back to Go.
  - `width("00") == 2 × width("0")`.
  - A rotated bitmap has swapped dimensions and known pixels in the right places.
- **Blending:** hand-derived expected values:
  - coverage 0 leaves the pixel unchanged;
  - coverage 255 writes the color;
  - 50% white over black gives 188;
  - the table matches the sRGB formula within ±1.
- **`wrap()`:** table tests with a fixed-width fake measure:
  - spaces and `\n`;
  - the line limits;
  - an ellipsis on the last line;
  - an over-long single word;
  - nothing fits.
- **Layout:**
  - No text overlaps other text at 800×600, 500×300 and 300×200 with a title, both names and both offsets. This extends `offset_labels_stay_clear_of_other_labels`.
  - Each collision rule stacks the right line.
  - A long title uses 3 lines.
  - Absent or empty text adds no band.
- **`ticks.rs`:** the expected strings use `−`.
- **Sizing:** `Terminal::text_size` for 34 px rows gives 28, and a tiny row height gives 8. PNGs default to 14.
- **CLI:**
  - `axis_names` against the agreed example table.
  - `Table::points` returns the right names for `-x 3`, `-y name` and `-x index`.

**Golden images** (`tests/golden.rs`)
- All 7 existing snapshots are regenerated and each is checked by eye.
- New scenes:
  - `title_and_axis_names`: title, both names, and both offsets;
  - `long_text_wraps`: wrapping and an ellipsis;
  - `light_background_text`: dark text on white.
- Comparison stays exact. If CI shows anti-aliased pixels differing between platforms, the fallback is to allow ±1 color level per pixel rather than raise the 0.1% pixel budget. The PR would say so.

**Property tests** (`tests/properties.rs`): random `TextStyle` sizes (1 to `MAX_FONT_SIZE`) and random titles and names never panic, and no drawn text pixel leaves the canvas. The titles and names include empty, whitespace-only, arbitrary Unicode and very long strings.

**CLI tests** (`tests/cli.rs`)
- `--title` and `--xlabel` change the PNG compared with a render without them.
- `--xlabel ""` removes a header name.
- Both `--font` errors.
- `--font-size 0` is rejected.

## Docs

- **README:** a "Titles and axis names" section with an image. All images are regenerated with `scripts/readme_images.sh`; the sine, cosine and Lissajous examples gain `x`/`y` names from their headers. Plus the License note.
- **CLAUDE.md:**
  - Rewrite "Text/Number Rendering" for `font.rs`, blending and the layout bands.
  - Update the pipeline diagram.
  - Remove the bitmap-font notes and the "no titles" known issue.
- **IMPROVEMENTS.md:** item 13 is partly done (the legend follows in spec 2), as are the `--title` parts of items 18 and 32.
- **CHANGELOG.md:** `## [0.3.0] - Unreleased` with Added, Changed (breaking) and Removed.

## Delivery

Three PRs to `main`, each green on its own (tests, clippy, fmt, docs, MSRV). No release until spec 2 is done, when 0.3.0 is tagged.

1. **Text engine swap, no new features.**
   - Font asset, charset, subset script, license files, and the `Cargo.toml` license field.
   - `Font`, `Canvas::blend`, the new `TextStyle`/`Label` API, and `numbers.rs` removed.
   - The Unicode minus.
   - Release archives with license files.
   - Regenerated snapshots and README images.
   - The version moves to 0.3.0, with an Unreleased changelog section.
2. **Titles and axis names.** The layout bands and the two-pass collision rule, `wrap()`, the `Plot`/`Graph`/`TerminalCanvas` additions, `Terminal::text_size` and terminal-matched sizing, and the new golden scenes.
3. **CLI and docs.** The flags, automatic names from headers, `--verbose` output, and the README, CLAUDE.md, IMPROVEMENTS.md and CHANGELOG updates.

## Out of scope

- The legend and per-series names (spec 2).
- Separate styles for the title versus the axis names; a bold font.
- Anti-aliased lines and markers.
- Math or rich text, and hyphenation.
- Text placed in data coordinates (matplotlib's `ax.text`).
- A braille or half-block fallback renderer.
- Multi-line tick labels.
