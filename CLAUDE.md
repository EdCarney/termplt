# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
cargo build                          # Build
cargo test                           # Run all tests (unit tests in modules + tests/{golden,properties,cli,pty}.rs)
cargo test plotting::graph          # Run tests for a specific module
cargo test draw_into_matches_get_mask # Run a single test by name
cargo clippy --all-targets -- -D warnings  # Lint (CI fails on any warning)
cargo fmt --check                    # Formatting (enforced in CI)
TERMPLT_UPDATE_SNAPSHOTS=1 cargo test --test golden  # Regenerate golden PNGs after an intentional rendering change (review them!)
PROPTEST_CASES=20000 cargo test --test properties    # Longer property-test run
cargo run -- --data "(1,1),(2,4)"    # Render a plot via the CLI (needs a Kitty-protocol terminal)
cargo run -- data.csv -o plot.png    # Write a PNG instead (no terminal needed; handy for checking output)
cargo test --no-default-features     # Library only, without the clap-based CLI (skips tests/cli.rs and tests/pty.rs)
cargo test --test pty                # CLI in a pseudo-terminal against a scripted fake terminal (Unix only)
cargo +1.88 test --locked            # MSRV check (rust-version in Cargo.toml; CI runs it)
```

CI (`.github/workflows/ci.yml`): build + test on Linux, macOS and Windows (also with `--no-default-features`), the same tests on the MSRV (Rust 1.88, `--locked`), clippy, `cargo doc` with `-D warnings` (the crate has `#![warn(missing_docs)]`, and README.md is the crate doc, so its Rust examples are doctests), rustfmt. One feature, `cli` (default), gates the binary and its `clap` dependency. No custom build scripts. Edition 2024 with let-chains, so `rust-version = "1.88"`; don't use newer std APIs without raising it (clippy's `incompatible_msrv` and the MSRV job catch this).

Releases (`.github/workflows/release.yml`): push a `v*` tag equal to the `Cargo.toml` version. It checks the tag, tests on every target, builds the binaries, attaches them with `SHA256SUMS` to a GitHub release, and runs `cargo publish` if the `CARGO_REGISTRY_TOKEN` secret exists. Record user-visible changes in `CHANGELOG.md`.

## Architecture

**termplt** is a Rust library for rendering 2D plots directly in Kitty-compatible terminals using the Kitty graphics protocol. User data of any numeric type is stored as `f64`, scaled to pixel coordinates, rendered to an in-memory RGB canvas, then PNG-encoded and transmitted via Kitty APC escape sequences.

Public API layers, top-down:
- `Plot` (`plot.rs`): one-call builder (`line`/`scatter`/`line_points`/`series`, limits, size, background, grid; `show`, `save_png`, `render`). The CLI uses it too.
- `plotting::{series::Series, graph::Graph, canvas::TerminalCanvas}` and the style types; `prelude` re-exports the common ones.
- `terminal`: `Terminal` (support check, size, tmux handling, display) plus re-exported `WindowSize`, `get_window_size`, `Image`, `PixelFormat`, `Transmission`, `Passthrough`, `query_support`, `TerminalCommandError`.
- `Error`/`Result` (`error.rs`): one `#[non_exhaustive]` enum for the whole crate.

`kitty_graphics`, `terminal_commands` and `plotting::{common, numbers, ticks}` are private; re-export what users need rather than making them public.

### Numeric Types (`plotting/common.rs`)

`Series`, `Graph` and `TerminalCanvas` are not generic: data is converted to `f64` at the boundary (`Series::new`, `from_xy`, `from_y`, `FromIterator<(x, y)>`, `From<Vec<(x, y)>>`, `From<(xs, ys)>`). `Graphable` covers every primitive integer and float type via the crate's `ToF64` trait (`as` casts), so `i64`/`u64`/`usize` work. `Point<T>`, `Limits<T>` and the internal `Line<T>` stay generic because the pixel math uses them with `u32`/`i32`; the `Convertable`/`UIntConvertable`/`IntConvertable`/`FloatConvertable` traits do clamped casts between those (`v.clamp(0.0, u32::MAX as f64) as u32`).

**Zero-span safety:** `Graph::view_limits()` pads zero-width dimensions (5% of the value, or ±0.5 around zero) before scaling, so single points and constant series are centered. As a fallback, `Graph::scale_with_view` maps a zero span to the middle of the target range.

### Rendering Pipeline (`canvas.rs` → `graph.rs`)

```
TerminalCanvas::draw()
  ├── graph.view_limits()            # finite data limits + explicit limits, clipped,
  │                                  #   5% margin on automatic axes, zero spans padded
  ├── layout()                       # y ticks (fit plot height) → left margin from widest y label
  │                                  #   → x ticks (fit plot width); bottom band for x labels;
  │                                  #   inset for markers/thick lines/axes → plot area
  ├── graph.scale_with_view(..)      # clip → shift-to-origin → proportional scale → shift-to-plot
  ├── grid_lines.get_mask_at(..)     # grid at tick positions (drawn first)
  ├── axes.get_mask(plot)            # axis lines just outside the plot area
  ├── series.draw_into(canvas)       # markers, then lines, straight into the canvas
  ├── Canvas::set_pixels()           # write RGB8 into the flat pixel buffer
  └── labels → get_mask → set_pixels # tick labels (bitmap font), drawn last
```

Ticks (`ticks.rs`): values are k × step with step ∈ {1, 2, 5} × 10^k (Heckbert); `fit_ticks` picks the densest count (≤ `MAX_TICKS`) whose labels don't overlap at the actual pixel size. Labels on an axis share decimal places; scientific notation when |v| ≥ 1e6 or step < 1e-4. A label color equal to the background is replaced with black/white. `get_drawable_limits()` returns the plot area from the same layout.

The `Drawable` trait (`fn get_mask(&self) -> Result<Vec<MaskPoints>>`) is implemented by `Series`, `Line`, `Marker`, `Label`, and `Graph`. Each returns pixel coordinates + colors; the canvas composites them.

### Limits and performance

`Graph` keeps explicit limits as `x_limits`/`y_limits: Option<(f64, f64)>`; `Graph::limits()` merges them with the data extent, and explicit limits also clip points. `Graph::visible()` leaves one NaN gap point where points were removed (outside the limits, or non-finite), and both drawing paths skip non-finite points and the segments touching them, so lines break instead of bridging. `Series::draw_into` is the fast path used by the canvas (flat RGB buffer, marker offsets stamped once per style, line discs stamped incrementally, repeated pixels skipped); it must produce exactly the pixels of `Drawable::get_mask` (a test checks every style), so change both together.

### Kitty Protocol (`kitty_graphics/`)

After rendering, the canvas bytes are PNG-encoded (`Image::png_from_rgb`, `f=100`: about 100x smaller than raw RGB) and sent via Kitty APC sequences. `encoding.rs` does custom RFC 4648 base64 (with padding). `kitty_cmds.rs` chunks to 4096-byte payloads, wraps each chunk in tmux DCS passthrough when `$TMUX` is set (`Passthrough`), and provides `query_support()` (an `a=q` query). `ctrl_seq.rs` provides protocol key=value formatting. Display commands send `q=2` so no replies are left in the input; `File`/`TempFile` paths are made absolute. `TermCommand` writes commands to stdout. `execute_with_response` writes queries to `/dev/tty` (the `CONIN$`/`CONOUT$` console on Windows, so piped stdin does not interfere) followed by a DA1 sentinel, and reads the reply with a timeout under an RAII raw-mode guard. It fails fast with `TerminalCommandError::Unsupported` when the DA1 reply arrives first.

`get_window_size()` (`window_ctrl.rs`) takes the size from the OS (`TIOCGWINSZ` via crossterm) and only queries what is missing (`CSI 14t` pixels, `18t` cells). `terminal::Terminal::connect` (`src/terminal.rs`; terminal and OS access goes through a private `Backend` trait so every branch is unit-tested with a fake) does the following, in order:
- checks stdout is a TTY;
- runs the graphics query, or in tmux checks `allow-passthrough`;
- gets the window size, falling back to an estimate from the cell count.

In tmux it draws with `C=1` and prints the newlines itself.

### Line Drawing (`line.rs`)

`BetweenPoints` lines use Bresenham's algorithm; thickness stamps a disc of radius `thickness` at each pixel (round joins). `Horizontal`/`Vertical` lines use range iteration; thickness shifts parallel copies. `LineStyle::Dashed` filters the ordered path with a 6-on/4-off pattern. `MarkerStyle::None` draws no marker.

### Text/Number Rendering (`text.rs`, `numbers.rs`)

Bitmap font: 10x11 pixel grids for `0-9`, `.`, `-`, `e`, ` `; other characters render as a placeholder box. Supports scaling (pixel replication) and padding. `num_to_str` uses decimal when `0.1^sig_figs < |x| < 10^sig_figs`, otherwise scientific notation, with trailing zero stripping.

### CLI (`src/bin/termplt/`)

- `cli.rs`: clap derive definition; `--completions <SHELL>` prints a `clap_complete` script (static: flags, `--marker`/`--line` possible values, color names via `ColorParser`, file-path hints). Style options (`--color`, `--marker`, `--line`, ...) are defaults for every series; old snake_case flags are hidden aliases.
- `series.rs`: `--series` spec parsing (`key=value` pairs; a `,`/`;` only splits when followed by `key=`, so `data=(1,2),(3,4)` works), style resolution, palette, marker/line name parsing.
- `data.rs`: inline point parsing and `Table` (CSV/TSV/whitespace, header detection, columns by name or 1-based index, `index` = row number, missing values skipped).
- `main.rs`: collects series (FILE args × y columns, then `--data`, then `--series`; piped stdin when nothing else is given, read once and cached) into a `Plot`, connects a `Terminal` (errors get a `--output` hint) or writes a PNG with `--output`.

## Testing

- Unit tests are co-located in modules (`#[cfg(test)] mod tests`).
- `tests/golden.rs` renders fixed scenes and compares them with `tests/snapshots/*.png` (≤0.1% of pixels may differ). After an intentional visual change, regenerate with `TERMPLT_UPDATE_SNAPSHOTS=1` and inspect the PNGs before committing; mismatches are written to `target/snapshots/`.
- `tests/properties.rs` (proptest) checks that drawing never panics for arbitrary data/styles/sizes (including NaN/∞/extremes) and that scaled points stay within the target limits. Run in debug mode too — release builds disable integer-overflow checks.
- `tests/cli.rs` runs the CLI binary with piped stdio: PNG output from files and stdin, the not-a-terminal error, argument and data errors.
- `tests/pty.rs` (Unix) runs the CLI on a pty (`openpty`, made the controlling terminal so `/dev/tty` works) and plays a fake terminal on the master side: it answers the graphics query, `CSI 14t`/`18t` and DA1 as configured, and the tests decode the transmitted PNG and check its size, the control keys (`a=T`, `f=100`, `q=2`, `C=1` in tmux), what follows the image, and the error text. tmux is simulated with `$TMUX` and a fake `tmux` script on `PATH`. The terminal-setup decisions are also unit-tested with a fake `Backend` in `src/terminal.rs`, and reply parsing with a fake `ByteSource` in `responses.rs`.
- Use `tempfile` for files in tests, never fixed names in the shared temp directory.

## Known Issues

See `IMPROVEMENTS.md` for the full prioritized list and status. Key open items:
- Clipping to explicit limits drops points (the line breaks there) rather than clipping line segments at the boundary
- `Limits::new` panics on inverted bounds (internal invariant); use `Limits::try_new` for untrusted input
- Bitmap font only covers `0-9 . - e`, so there are no titles, axis names or legends yet
