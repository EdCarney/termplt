# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/) (before 1.0, a minor version bump may break the API).

## [0.3.0] - Unreleased

Text is drawn with an embedded TrueType font. The low-level text API has **breaking changes**;
see "Changed" and "Removed".

### Added
- `plotting::font::Font` (also in the prelude): the built-in Go font by default, or any
  TrueType or OpenType font with `Font::from_bytes`. Characters a font lacks are drawn with Go.
- `TerminalCanvas::with_font` and `with_font_size`, `text::DEFAULT_FONT_SIZE` (14 px) and
  `text::MAX_FONT_SIZE` (400 px).
- `Error::InvalidFont`.
- Release archives include `LICENSE` and the font's `LICENSE-Go`.
- Titles and axis names: `Plot::title`, `x_label` and `y_label`, and `Graph::with_title`,
  `with_x_label` and `with_y_label`. Long text wraps (titles onto 3 lines, names onto 2) and
  ends with `…` when it still doesn't fit; the y name reads upwards.
- `Plot::font` and `Plot::font_size`. `Plot::show` matches the terminal's text size
  (`Terminal::text_size`); PNGs use 14 px.
- CLI: `--title`, `--xlabel`, `--ylabel`, `--font` and `--font-size`. Axes are named from CSV
  headers when every series has the same one (`--xlabel ""` removes a name). In a terminal the
  text matches the terminal's own size; with `--output` it is 14 px.
- Legends: `Series::with_label` names a series, and a graph with named series shows a legend
  inside the plot, framed and 80% opaque as in matplotlib, at the fixed location that covers
  the least data (matplotlib's `loc="best"`). `Graph::with_legend(false)` / `Plot::legend(false)`
  hide it; `Graph::with_legend_location` / `Plot::legend_location` pick one of matplotlib's ten
  locations (`plotting::legend::LegendLocation`, also in the prelude). Long names wrap onto 2
  lines, and series that don't fit are counted in a `+N more` row.
- CLI legend: series are named from `--series ...,label=NAME`, else the y column's header, else
  the file name (`stem: column` for a file giving several series; colliding names fall back to
  file names). `--legend`, `--no-legend` and `--legend-loc` (matplotlib's location names).

### Changed
- All text, tick labels included, is drawn with an embedded, trimmed copy of the Go font
  (BSD-3-Clause), anti-aliased and blended in linear light. It covers Latin-1, Greek and common
  math symbols. The crate's license is now `MIT AND BSD-3-Clause`.
- Numbers use the Unicode minus sign `−`, as matplotlib does.
- Gaps around tick labels, names and the title scale with the text size (matplotlib's
  paddings), and y tick labels are centered on their digits.
- **Breaking:** `TextStyle::new(color, size_px)` replaces `TextStyle::new(color, scale,
  padding)`, and `TextStyle::size()` replaces `scale()` and `padding()`.
  `TextStyle::with_color` is unchanged; its text takes the canvas's base size.
- **Breaking:** `Label::new(text, style, pos)` with `text()`, `style()` and `pos()` replaces
  `Label::new(Text, TextPositioning)`, `txt()` and `limits()`. `Label` no longer implements
  `Drawable`.
- CLI plots with 2 or more series show a legend; `--no-legend` restores the old look.

### Removed
- **Breaking:** `Text`, `MAX_TEXT_SCALE` and `MAX_TEXT_PADDING` (the bitmap font).

## [0.2.1] - 2026-09-25

### Fixed
- Tick labels on axes far from zero. Near 1e15, where f64 values are 0.125 apart, labels showed
  wrong values (`…0.2`, `…0.5`, `…0.6`) at uneven spacing, and one could appear below the axis.
  Such axes now get an offset, as in matplotlib: the ticks are fitted and placed relative to it,
  so they are exact and evenly spaced, and the offset is shown once (`+1e15`) under the right
  end of the x axis or above the y axis.

### Changed
- Any axis whose values share at least 4 leading digits gets an offset (matplotlib's default
  threshold), so timestamps like `1700000000 1700000050 1700000100` are now labeled
  `0 50 100` with `+1.7e9`. Unlike matplotlib, the offset shows all of its digits rather than
  rounding to 10.

## [0.2.0] - 2026-09-25

A large rework of the library and CLI. The library API has **breaking changes**; see
"Changed" and "Removed".

### Added
- `Plot`, a one-call builder: `line`, `scatter`, `line_points`, `series`, `x_limits`, `y_limits`,
  `size`, `background`, `grid`, then `show()`, `show_in()`, `save_png()` or `render()`.
- `terminal::Terminal`: checks for Kitty graphics support (an `a=q` query with a DA1 sentinel),
  finds the window size, handles tmux, and displays images. `query_support()` is public too.
- `termplt::prelude` with the commonly used types.
- `termplt::Error`, a `#[non_exhaustive]` error enum (`Send + Sync`), and `termplt::Result`.
- `Series::from_xy`, `Series::from_y`, `FromIterator<(x, y)>`, and `From` for `Vec<(x, y)>`,
  `&[(x, y)]`, `(xs, ys)` pairs of slices or `Vec`s, and `Vec<Point<T>>`, with any primitive
  numeric type (including `i64`, `u64` and `usize`).
- `colors::PALETTE`, `colors::luminance`, `colors::from_name` and `colors::parse` (hex).
- `LineStyle::solid`, `LineStyle::dashed` and `Default` impls for `LineStyle`, `Graph`,
  `Series` and `Plot`; `MarkerStyle::None`.
- tmux support: images are sent through DCS passthrough; a clear error explains how to enable
  `allow-passthrough` when it is off.
- CLI: reads piped stdin (or `-`); column selection with `-x`/`-y` by name or 1-based index;
  per-series styles with `-s/--series`; `--output` (PNG), `--width`, `--height`, `--xlim`,
  `--ylim`, `--bg`, `--no-grid`, `--list-colors`, `--list-markers`, `--completions <SHELL>`,
  `--version`; missing values (`NA`, empty, `null`, ...) are skipped with a count.
- Nice tick values with labels sized to fit, a 5% data margin, and grid lines under the axes.
- Golden-image and property tests; end-to-end CLI tests, including a pseudo-terminal harness on
  Unix that plays a Kitty-like, silent, graphics-less or tmux terminal; rustdoc for the whole
  public API (README examples are doctests).
- `rust-version = "1.88"`. CI tests Linux, macOS, Windows and the minimum Rust version, and
  checks docs, clippy and formatting.
- Release workflow: runs on a pushed `v*` tag, checks that it matches the `Cargo.toml` version,
  runs the tests on every target before building, names archives with the version, attaches a
  `SHA256SUMS` file, and publishes to crates.io when a `CARGO_REGISTRY_TOKEN` secret is set.

### Changed
- `Series`, `Graph` and `TerminalCanvas` are no longer generic: data is stored as `f64`.
- Library functions return `termplt::Result` instead of `Result<_, Box<dyn Error>>`.
- Images are sent as PNG (`f=100`), about 100x smaller than raw RGB, with `q=2` so the terminal
  sends no replies.
- The window size comes from the OS (`TIOCGWINSZ`); terminal queries are only used for missing
  parts, and are written to `/dev/tty` (the console on Windows) with a timeout, so the CLI no
  longer hangs on a silent terminal or when stdin is piped.
- The default plot size fits the terminal (full width, 60% of the height, at most 2:1).
- `line_thickness` now thickens series lines.
- Rendering is 2-14x faster for large series (1M points in about 0.1-0.5 s), with identical
  pixels.
- CLI flags use kebab-case (`--marker-size`); the old snake_case spellings still work.
- `image` is built with only PNG support; `--output` accepts only `.png`.
- `WindowSize` and `get_window_size` moved from the crate root to `termplt::terminal`.
- Lines break where points were dropped (outside explicit limits, or NaN/±∞) instead of joining
  the neighbouring points.
- Hex colors need their `#` (`colors::parse`, `--color`): words such as `bad` or `facade` are
  no longer taken for hex.
- `--marker-size`, `--line-thickness` and the matching `--series` keys accept 0 to 100;
  `TextStyle::new` clamps the scale to 32 and the padding to 64.
- A terminal that reports no size at all is assumed to be 80x24 cells (with a warning) instead
  of being an error.
- `GridLines::XOnly`/`YOnly` are documented as what they always drew: lines parallel to the
  named axis (horizontal for `XOnly`).
- `Error::source` returns the wrapped error's source, since the wrapped message is already part
  of the message; error reporters no longer print it twice.
- `Text::from_number` rounds to the requested significant figures instead of truncating.
- `PartialEq` for `Plot`, `Graph`, `Series`, `Axes`, `Image` and the style types; `Copy` for
  `TextStyle`, `TextPositioning`, `AxesPositioning` and `GridLines`; `Eq` where possible.
  `TerminalCommandError` is `#[non_exhaustive]`. `Transmission`'s `Debug` shows the size of
  inline data instead of every byte.

### Removed
- The `kitty_graphics` and `terminal_commands` modules and `plotting::{common, numbers, ticks}`
  are private; what users need is re-exported from `termplt::terminal` and `termplt::plotting`.
- `Scalable`, `Shiftable`, `GraphLimits`, `DisplayRegion`, `StackingOrder`,
  `Positioning::Current`, `clear_screen`, and the `png_imgs`/`rgb_imgs`/`rgba_imgs` demo helpers.

### Fixed
- Panics on NaN/infinite data, on explicit limits that exclude all data, on unexpected terminal
  replies and on several invalid inputs (non-finite values are now skipped).
- A single point or a constant series is centered instead of drawn at the edge.
- `Limits::intersects` missed overlaps where neither box contained a corner of the other.
- CSV error line numbers were off by one after a header row.
- Running with stdout redirected no longer writes escape codes into the pipe; it reports that
  stdout is not a terminal and suggests `--output`.
- Tick labels for data with a large offset (e.g. timestamps) were all identical
  (`1.700000e9`), and end labels could run into their neighbours.
- Overflow panics in the layout with huge marker sizes, line or axis thicknesses or buffers,
  and in `Label` near `u32::MAX`; a division by zero for terminals reporting fewer pixels than
  cells.
- On macOS the CLI hung forever when the terminal answered no query: `poll()` doesn't support
  `/dev/tty` there, so the 2 s timeout never fired. Queries now wait with `select()` on macOS.
- On Windows, VT input is enabled on the console while a query runs, so replies can arrive.
- Query timeouts no longer claim a Kitty terminal is required when the query was for the size.
- CSV: a UTF-8 byte order mark no longer makes the first row a header; semicolon-separated
  files parse; a single column with `-y 1` is plotted against the row number.
- `Series` converts from arrays of `(x, y)` tuples, `&Vec<(x, y)>`, pairs of arrays and
  `&[Point<T>]`, as the docs suggested.

## [0.1.2] - 2026-02-16

### Fixed
- Base64 payloads are padded correctly.
- Window size lookup falls back to `CSI` queries (for Windows terminals).

## [0.1.1] - 2026-02-15

First releases (0.1.0 and 0.1.1, published the same day).

[0.3.0]: https://github.com/EdCarney/termplt/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/EdCarney/termplt/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/EdCarney/termplt/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/EdCarney/termplt/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/EdCarney/termplt/releases/tag/v0.1.1
