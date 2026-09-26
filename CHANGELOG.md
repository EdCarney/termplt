# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows
[Semantic Versioning](https://semver.org/) (before 1.0, a minor version bump may break the API).

## [0.2.0] - Unreleased

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

## [0.1.2] - 2026-02-16

### Fixed
- Base64 payloads are padded correctly.
- Window size lookup falls back to `CSI` queries (for Windows terminals).

## [0.1.1] - 2026-02-15

First releases (0.1.0 and 0.1.1, published the same day).

[0.2.0]: https://github.com/EdCarney/termplt/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/EdCarney/termplt/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/EdCarney/termplt/releases/tag/v0.1.1
