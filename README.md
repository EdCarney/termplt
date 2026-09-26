# termplt

A Rust library for rendering 2D plots directly in [Kitty](https://sw.kovidgoyal.net/kitty/)-compatible terminals. Data goes in, pixel-perfect graphs come out; no GUI, no image files, no browser!

termplt uses the [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) to transmit rendered plots (PNG-compressed, so they are fast over SSH too) via APC escape sequences, so graphs display inline in your terminal.

## Features

- **One call to plot** — `Plot::new().line((&xs, &ys)).show()?` sizes itself to the terminal; `.save_png(path)` writes a file
- **Any numeric data** — `i32`, `i64`, `u64`, `usize`, `f32`, `f64`, ... given as `(xs, ys)`, `(x, y)` tuples or points
- **Multiple series** — overlay multiple data series on a single graph with independent styling
- **Marker styles** — filled/hollow circles and squares with configurable size and color
- **Line drawing** — optional solid or dashed connecting lines of any thickness
- **Axes and grid lines** — round tick values, with label space laid out automatically
- **Axis limits** — optionally constrain x/y ranges with automatic point clipping
- **Configurable canvas** — set dimensions, background color, and buffer padding
- **Bitmap text** — built-in 10x11 pixel font for labels and numeric annotations
- **Typed errors** — match on `termplt::Error` (no data, canvas too small, terminal unsupported, ...)
- **Fast** — a million points render in about 0.1-0.3 s

## CLI Usage

termplt includes a command-line tool for plotting without writing Rust code. Data comes from files, stdin or inline points, and each series can have its own columns and style.

```bash
# Plot a CSV file (columns 1 and 2; a header row is detected automatically)
termplt data.csv

# Pipe data in
seq 0 50 | awk '{print $1/5, sin($1/5)}' | termplt

# Inline points
termplt --data "(1,1),(2,4),(3,9),(4,16)"
```
<img width="600" height="450" alt="Line plot of the points (1,1), (2,4), (3,9) and (4,16)" src="https://raw.githubusercontent.com/EdCarney/termplt/main/docs/images/inline-points.png" />

The `test_data/` files in the next examples come from `scripts/generate_test_data.py` (see [Test Data Generation](#test-data-generation)).

```bash
# Scatter plot (no connecting lines)
termplt test_data/random_clusters.csv --line none
```
<img width="600" height="450" alt="Scatter plot of three clusters of random points" src="https://raw.githubusercontent.com/EdCarney/termplt/main/docs/images/scatter.png" />

```bash
# Multiple series with automatic color cycling
termplt test_data/sine.csv test_data/cosine.csv

# Several columns of one file, selected by header name
termplt sensors.csv -x time -y temp,humidity
```
<img width="600" height="450" alt="Sine and cosine curves in two palette colors" src="https://raw.githubusercontent.com/EdCarney/termplt/main/docs/images/multiple-series.png" />

```bash
# Custom styling for every series
termplt test_data/lissajous.csv --marker hollow-circle --marker-size 4 --color cyan --line-thickness 1
```
<img width="600" height="450" alt="Cyan Lissajous curve with hollow circle markers" src="https://raw.githubusercontent.com/EdCarney/termplt/main/docs/images/custom-style.png" />

```bash
# Per-series data and style with --series
termplt -s "file=a.csv,color=red" -s "file=b.csv,color=#1e90ff,marker=none,line=dashed"

# Line-only plot, axis limits, and write a PNG instead of displaying it
termplt test_data/noisy_linear.csv --marker none --color lime --line-thickness 1 --xlim 0,10 -o plot.png
```
<img width="600" height="450" alt="Green line of noisy linear data with the x axis limited to 0 to 10" src="https://raw.githubusercontent.com/EdCarney/termplt/main/docs/images/line-limits.png" />

### Options

| Option | Description |
|---|---|
| `FILE...` | Data files (CSV, TSV or whitespace-delimited); `-` reads stdin. Piped stdin is read automatically when no other data is given |
| `-d, --data <POINTS>` | Inline points: `"(1,2),(3,4)"`, `"1,2 3,4"` or `"1,2;3,4"` |
| `-s, --series <SPEC>` | A series with its own data, columns and style (see below) |
| `-x, --x-col <COL>` | Column for x: header name, 1-based index, or `index` for the row number (default: 1) |
| `-y, --y-col <COLS>` | Column(s) for y, comma-separated for one series each (default: 2) |
| `-c, --color <COLOR>` | Color for markers and lines: a name (`red`, `DarkRed`, `dark-red`) or `#RRGGBB` |
| `--marker <STYLE>` | `filled-circle`, `hollow-circle`, `filled-square`, `hollow-square`, `none` |
| `--marker-size <PX>` / `--marker-color <COLOR>` | Marker radius (default: 2) and color |
| `--line <STYLE>` | `solid` (default), `dashed`, or `none` (scatter plot) |
| `--line-thickness <PX>` / `--line-color <COLOR>` | Line thickness (default: 0) and color |
| `--xlim <MIN,MAX>` / `--ylim <MIN,MAX>` | Axis limits; points outside are not drawn |
| `--width <PX>` / `--height <PX>` | Image size (default: fits the terminal, or 800x600 with `--output`) |
| `--bg <COLOR>` | Background color (default: black) |
| `--no-grid` | Hide grid lines |
| `-o, --output <FILE>` | Write a PNG file (e.g. `plot.png`) instead of displaying; no terminal needed |
| `--list-colors` / `--list-markers` | List color names / marker styles |
| `-v, --verbose` | Print terminal size, canvas and plot area to stderr |
| `--completions <SHELL>` | Print a completion script for `bash`, `zsh`, `fish`, `powershell` or `elvish` |
| `-h, --help` / `-V, --version` | Help / version |

Style options apply to every series. Unset colors and markers cycle through a palette.

A `--series` spec is a list of `key=value` pairs separated by commas: `file=PATH` or `data=POINTS` (required), `x=COL`, `y=COL`, `color`, `marker`, `marker-size`, `marker-color`, `line`, `line-color` and `line-thickness`. Keys override the options above for that series. Inline data keeps its commas, e.g. `-s "data=(1,2),(3,4),color=red"`.

Data files may have a header row (detected automatically), `#` comments and blank lines. A single-column file is plotted against the row number. Rows with missing values (empty, `NA`, `null`, ...) and NaN/infinite values are skipped with a warning.

### Shell completions

Tab completion covers flags, marker and line styles, color names and file paths. Generate a script for your shell once (and again after upgrading):

```bash
# bash
mkdir -p ~/.local/share/bash-completion/completions
termplt --completions bash > ~/.local/share/bash-completion/completions/termplt

# zsh: add `fpath=(~/.zfunc $fpath)` to ~/.zshrc before `compinit` runs
# (with oh-my-zsh, before `source $ZSH/oh-my-zsh.sh`)
mkdir -p ~/.zfunc
termplt --completions zsh > ~/.zfunc/_termplt
# or, with Homebrew, into a directory zsh already searches:
# termplt --completions zsh > "$(brew --prefix)/share/zsh/site-functions/_termplt"

# fish
mkdir -p ~/.config/fish/completions
termplt --completions fish > ~/.config/fish/completions/termplt.fish
```

Start a new shell afterwards (for zsh, `rm -f ~/.zcompdump*` first if completions don't appear). Values that depend on your data, such as column names, are not completed.

Flag spellings from earlier versions (`--data_file`, `--marker_style`, `--line_thickness`, ...) are still accepted.

## Requirements

- A terminal implementing the [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) (see below)
- Rust 2024 edition (Rust 1.88 or later)

### Terminal support

| Terminal | Status |
|---|---|
| Kitty, Ghostty, WezTerm | Supported |
| Konsole (22.04+) | Implements the protocol; less tested |
| Inside tmux 3.3+ | Supported with `set -g allow-passthrough on` (see below) |
| GNU screen, Alacritty, GNOME Terminal, Terminal.app, iTerm2, Windows Terminal, xterm | Not supported: use `--output plot.png` |

The CLI checks for graphics support before drawing (a Kitty `a=q` query) and says so instead of printing garbage when the terminal lacks it. Plots are sent as PNG, so a full-screen plot is tens of kilobytes rather than megabytes, which keeps it quick over SSH. The terminal size comes from the operating system (`TIOCGWINSZ`) when it reports pixel sizes, otherwise from `CSI 14t`/`18t` queries, otherwise it is estimated from the number of rows and columns (with a warning; set `--width`/`--height` if the plot comes out the wrong size).

**tmux.** termplt wraps its output in tmux's passthrough sequence, which tmux only forwards when allowed:

```bash
tmux set -g allow-passthrough on          # current server
echo 'set -g allow-passthrough on' >> ~/.tmux.conf   # permanently
```

The CLI stops with that hint when passthrough is off. tmux doesn't track the image itself, so it disappears when tmux redraws the pane (switching windows, resizing, scrolling in copy mode); run the command again to redraw it.

**Windows.** Rendering works in terminals that implement the protocol on Windows, such as WezTerm. Terminal queries are read from the console (`CONIN$`) with VT input; this path is compiled and unit-tested in CI but has not been verified interactively yet. Windows Terminal does not implement the Kitty protocol, so use `--output` there. If you try it, please report what works:

1. `termplt --data "(1,1),(2,4)" -v` shows a plot and prints the detected terminal size.
2. `Get-Content data.csv | termplt -v` (PowerShell) works with piped input (queries still reach the console).
3. The console is left in its normal mode afterwards (typing echoes, Ctrl-C works).

## Quick Start

Add `termplt` to your `Cargo.toml`:

```toml
[dependencies]
termplt = { version = "0.2", default-features = false }
```

The default `cli` feature builds the command-line tool; `default-features = false` skips its dependencies when you only need the library.

### Plotting in one call

```rust,no_run
use termplt::prelude::*;

fn main() -> termplt::Result<()> {
    let xs: Vec<f64> = (0..=200).map(|i| i as f64 * 0.05).collect();
    let sin: Vec<f64> = xs.iter().map(|x| x.sin()).collect();
    let cos: Vec<f64> = xs.iter().map(|x| x.cos()).collect();

    Plot::new()
        .line((&xs, &sin)) // a line in the first palette color
        .line_points((&xs, &cos)) // a line with markers in the next one
        .scatter(vec![(2, 0.5), (7, -0.5)]) // any numeric types
        .show()?; // sized to fit the terminal

    // or write a PNG; no terminal needed
    Plot::new()
        .line((&xs, &sin))
        .size(800, 600)
        .save_png("sine.png")?;
    Ok(())
}
```

`Plot` draws axes with tick labels and a grid, picks colors from `colors::PALETTE`, and adapts the axes to light backgrounds (`.background(colors::WHITE)`). Other options: `.x_limits(min, max)`, `.y_limits(min, max)`, `.grid(false)`, and `.series(s)` for a series with your own styles.

### Full control

`Plot` is built on lower-level types that you can use directly: `Series` (points and styles), `Graph` (series, axes, grid, limits) and `TerminalCanvas` (pixel size, background, margins).

```rust
use termplt::prelude::*;

fn main() -> termplt::Result<()> {
    let series = Series::from_xy(&[1, 2, 3, 4], &[1.0, 4.0, 9.0, 16.0])
        .with_marker_style(MarkerStyle::HollowCircle { size: 3, color: colors::ORANGE })
        .with_line_style(LineStyle::dashed(colors::ORANGE, 1));

    let graph = Graph::new()
        .with_series(series)
        .with_y_limits(0, 20)
        .with_axes(Axes::new(
            AxesPositioning::XY(LineStyle::solid(colors::WHITE, 1)),
            TextStyle::with_color(colors::WHITE),
        ))
        .with_grid_lines(GridLines::XY(LineStyle::solid(colors::DIM_GRAY, 0)));

    // RGB8 pixels, row-major from the top row
    let rgb = TerminalCanvas::new(640, 480, colors::BLACK)
        .with_buffer(BufferType::Uniform(12))
        .with_graph(graph)
        .draw()?
        .into_bytes();
    assert_eq!(rgb.len(), 640 * 480 * 3);

    // to display them: Terminal::connect()?.show_rgb(&rgb, 640, 480)?;
    Ok(())
}
```

### Errors

Every fallible call returns `termplt::Result<T>`. `termplt::Error` says what went wrong, so you can, for example, fall back to a file when the terminal can't show images:

```rust,no_run
use termplt::{Error, Plot};

let plot = Plot::new().line(vec![(0, 0), (1, 1)]);
match plot.show() {
    Err(Error::NotATerminal | Error::GraphicsUnsupported | Error::TmuxPassthroughDisabled) => {
        plot.save_png("plot.png").expect("cannot write plot.png");
        eprintln!("this terminal can't show images; wrote plot.png");
    }
    other => other.expect("cannot draw the plot"),
}
```

## Architecture

The rendering pipeline flows through four stages:

```text
User data (any numeric type, stored as f64)
  → Scale to pixel coordinates
    → Render to an in-memory canvas (RGB pixel buffer)
      → PNG-encode and transmit via Kitty APC escape sequences
```

Key abstractions:

| Module | Purpose |
|---|---|
| `Plot` | One-call builder: series, limits, size, background; `show`, `save_png`, `render` |
| `prelude` | The types most plots need |
| `plotting::series` | `Series`: data points with marker and line styles, built from any numeric input |
| `plotting::graph` | `Graph`: series, axes, grid lines and limits |
| `plotting::canvas` | `TerminalCanvas`: layout (ticks, labels, margins) and rendering to RGB pixels |
| `terminal` | `Terminal` (support check, size, tmux handling, display) and `Image` (Kitty protocol) |
| `Error` | Everything that can go wrong |

## Building and Testing

```bash
cargo build       # Build the library and CLI binary
cargo test        # Run unit, golden-image, property and end-to-end CLI tests
cargo clippy --all-targets -- -D warnings   # Lint
cargo run -- data.csv                       # Render a plot (requires a Kitty-compatible terminal)
cargo run -- data.csv -o plot.png           # Or write it to an image file
```

The end-to-end tests run the CLI binary: `tests/cli.rs` writes PNGs and checks errors, and on Unix `tests/pty.rs` runs it in a pseudo-terminal against a scripted fake terminal (Kitty-like, silent, no graphics, tmux with passthrough on and off), so the terminal handling is tested without a real terminal. CI runs the tests on Linux, macOS and Windows, and on the minimum supported Rust version (1.88).

Releases are made by pushing a tag that matches the version in `Cargo.toml` (`git tag v0.2.1 && git push origin v0.2.1`). The release workflow tests, builds binaries for Linux, macOS (Intel and Apple Silicon) and Windows, attaches them with a `SHA256SUMS` file to a GitHub release, and publishes to crates.io when the `CARGO_REGISTRY_TOKEN` secret is set. See [CHANGELOG.md](https://github.com/EdCarney/termplt/blob/main/CHANGELOG.md) for what changed in each version.

### Test Data Generation

A Python script is included to generate sample data files for validating CLI behavior:

```bash
python3 scripts/generate_test_data.py
```

This creates 13 data files in `test_data/` covering sine/cosine, polynomials, exponentials, parametric curves (circle, Lissajous), random scatter, Gaussian clusters, and more. The script prints example `cargo run` commands for each dataset.

## License

This project is licensed under the [MIT License](https://github.com/EdCarney/termplt/blob/main/LICENSE).
