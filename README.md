# termplt

A Rust library for rendering 2D plots directly in [Kitty](https://sw.kovidgoyal.net/kitty/)-compatible terminals. Data goes in, pixel-perfect graphs come out; no GUI, no image files, no browser!

termplt uses the [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) to transmit rendered plots as RGB pixel data via APC escape sequences, so graphs display inline in your terminal.

## Features

- **Generic numeric types** — plot `i32`, `u32`, `f32`, `f64`, or any type satisfying basic arithmetic traits
- **Multiple series** — overlay multiple data series on a single graph with independent styling
- **Marker styles** — filled/hollow circles and squares with configurable size and color
- **Line drawing** — optional solid or dashed connecting lines of any thickness
- **Axes and grid lines** — round tick values, with label space laid out automatically
- **Axis limits** — optionally constrain x/y ranges with automatic point clipping
- **Configurable canvas** — set dimensions, background color, and buffer padding
- **Bitmap text** — built-in 10x11 pixel font for labels and numeric annotations
- **Image display** — render PNG, RGB, and RGBA images inline via Kitty protocol

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
<img width="517" height="518" alt="image" src="https://github.com/user-attachments/assets/64f9f8a0-3631-4c6f-baf4-782694f673e1" />

```bash
# Scatter plot (no connecting lines)
termplt test_data/random_clusters.csv --line none
```
<img width="517" height="518" alt="image" src="https://github.com/user-attachments/assets/97be7e79-5224-44c3-b9a3-e7676887ef4e" />

```bash
# Multiple series with automatic color cycling
termplt test_data/sine.csv test_data/cosine.csv

# Several columns of one file, selected by header name
termplt sensors.csv -x time -y temp,humidity
```
<img width="517" height="518" alt="image" src="https://github.com/user-attachments/assets/4a9a3f03-2ff5-452f-a904-3006e8b4f3ed" />

```bash
# Custom styling for every series
termplt test_data/lissajous.csv --marker hollow-circle --marker-size 4 --color cyan --line-thickness 1
```
<img width="517" height="518" alt="image" src="https://github.com/user-attachments/assets/24d56a94-9013-40bf-b4cb-eaa38082c7a6" />

```bash
# Per-series data and style with --series
termplt -s "file=a.csv,color=red" -s "file=b.csv,color=#1e90ff,marker=none,line=dashed"

# Line-only plot, axis limits, and write a PNG instead of displaying it
termplt data.csv --marker none --color lime --line-thickness 1 --xlim 0,10 -o plot.png
```
<img width="517" height="518" alt="image" src="https://github.com/user-attachments/assets/18e2f854-94c1-4a8a-ae4e-c446683488cf" />

_The screenshots above are from an earlier version; axes now use round tick values._

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
| `-o, --output <FILE>` | Write an image file (e.g. `plot.png`) instead of displaying; no terminal needed |
| `--list-colors` / `--list-markers` | List color names / marker styles |
| `-v, --verbose` | Print terminal size, canvas and plot area to stderr |
| `--completions <SHELL>` | Print a completion script for `bash`, `zsh`, `fish`, `powershell` or `elvish` |
| `-h, --help` / `-V, --version` | Help / version |

Style options apply to every series. Unset colors and markers cycle through a palette.

A `--series` spec is a list of `key=value` pairs separated by commas: `file=PATH` or `data=POINTS` (required), `x=COL`, `y=COL`, `color`, `marker`, `marker-size`, `marker-color`, `line`, `line-color` and `line-thickness`. Keys override the options above for that series. Inline data keeps its commas, e.g. `-s "data=(1,2),(3,4),color=red"`.

Data files may have a header row (detected automatically), `#` comments and blank lines. A single-column file is plotted against the row number. Rows with missing values (empty, `NA`, `null`, ...) and NaN/infinite values are skipped with a warning.

### Shell completions

Tab completion covers flags, marker and line styles, and file paths. Generate a script for your shell once (and again after upgrading):

```bash
# bash
termplt --completions bash > ~/.local/share/bash-completion/completions/termplt

# zsh: any directory in $fpath, e.g. ~/.zfunc with `fpath=(~/.zfunc $fpath)` before `compinit` in ~/.zshrc
termplt --completions zsh > ~/.zfunc/_termplt

# fish
termplt --completions fish > ~/.config/fish/completions/termplt.fish
```

Start a new shell afterwards. Values that depend on your data, such as column names, are not completed.

Flag spellings from earlier versions (`--data_file`, `--marker_style`, `--line_thickness`, ...) are still accepted.

## Requirements

- A Kitty-compatible terminal (Kitty, WezTerm, or any terminal supporting the [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/))
- Rust 2024 edition

## Quick Start

Add `termplt` to your `Cargo.toml`:

```toml
[dependencies]
termplt = { version = "0.1", default-features = false }
```

The default `cli` feature builds the command-line tool; `default-features = false` skips its dependencies when you only need the library.

### Plotting a sine wave

```rust
use std::f32;
use termplt::plotting::{
    axes::{Axes, AxesPositioning},
    canvas::{BufferType, TerminalCanvas},
    colors,
    graph::Graph,
    grid_lines::GridLines,
    line::LineStyle,
    marker::MarkerStyle,
    point::Point,
    series::Series,
    text::TextStyle,
};
use termplt::kitty_graphics::ctrl_seq::{PixelFormat, Transmission};
use termplt::terminal_commands::{images::Image, responses::TermCommand};

fn main() {
    let num_points = 100;
    let points: Vec<Point<f32>> = (0..=num_points)
        .map(|x| {
            let x = (x as f32) * (360. / (num_points as f32));
            Point::new(x, (x * f32::consts::PI / 180.).sin())
        })
        .collect();

    let width = 800;
    let height = 600;
    let bytes = TerminalCanvas::new(width, height, colors::BLACK)
        .with_buffer(BufferType::Uniform(80))
        .with_graph(
            Graph::new()
                .with_series(
                    Series::new(&points)
                        .with_marker_style(MarkerStyle::FilledCircle {
                            size: 2,
                            color: colors::LIME,
                        })
                        .with_line_style(LineStyle::Solid {
                            color: colors::LIME,
                            thickness: 0,
                        }),
                )
                .with_axes(Axes::new(
                    AxesPositioning::XY(LineStyle::Solid {
                        color: colors::GHOST_WHITE,
                        thickness: 1,
                    }),
                    TextStyle::with_color(colors::WHITE),
                ))
                .with_grid_lines(GridLines::XY(LineStyle::Solid {
                    color: colors::GRAY,
                    thickness: 0,
                })),
        )
        .draw()
        .unwrap()
        .get_bytes();

    Image::new(
        PixelFormat::Rgb { width, height },
        Transmission::Direct(bytes),
    )
    .unwrap()
    .display()
    .unwrap();
}
```

## Architecture

The rendering pipeline flows through four stages:

```
User data (generic T: Graphable)
  → Scale to pixel coordinates
    → Render to in-memory canvas (RGB pixel buffer)
      → Transmit via Kitty APC escape sequences
```

Key abstractions:

| Module | Purpose |
|---|---|
| `plotting::common` | `Graphable` trait, type conversion, coordinate transforms |
| `plotting::graph` | `Graph` — composes series, axes, grid lines, and limits |
| `plotting::canvas` | `TerminalCanvas` — orchestrates rendering to pixel buffer |
| `plotting::series` | `Series` — data points with marker and line styles |
| `kitty_graphics` | Kitty protocol encoding and command chunking |
| `terminal_commands` | Image display and terminal interaction |

## Building and Testing

```bash
cargo build       # Build the library and CLI binary
cargo test        # Run unit, golden-image and property tests
cargo clippy --all-targets -- -D warnings   # Lint
cargo run -- data.csv                       # Render a plot (requires a Kitty-compatible terminal)
cargo run -- data.csv -o plot.png           # Or write it to an image file
```

### Test Data Generation

A Python script is included to generate sample data files for validating CLI behavior:

```bash
python3 scripts/generate_test_data.py
```

This creates 13 data files in `test_data/` covering sine/cosine, polynomials, exponentials, parametric curves (circle, Lissajous), random scatter, Gaussian clusters, and more. The script prints example `cargo run` commands for each dataset.

## License

This project is licensed under the [MIT License](LICENSE).
