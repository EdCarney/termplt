# termplt

A Rust library for rendering 2D plots directly in [Kitty](https://sw.kovidgoyal.net/kitty/)-compatible terminals. Data goes in, pixel-perfect graphs come out — no GUI, no image files, no browser.

termplt uses the [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) to transmit rendered plots as RGB pixel data via APC escape sequences, so graphs display inline in your terminal.

## Features

- **Generic numeric types** — plot `i32`, `u32`, `f32`, `f64`, or any type satisfying basic arithmetic traits
- **Multiple series** — overlay multiple data series on a single graph with independent styling
- **Marker styles** — filled/hollow circles and squares with configurable size and color
- **Line drawing** — solid connecting lines between points using Bresenham's algorithm
- **Axes and grid lines** — automatic axis rendering with numeric tick labels
- **Axis limits** — optionally constrain x/y ranges with automatic point clipping
- **Configurable canvas** — set dimensions, background color, and buffer padding
- **Bitmap text** — built-in 10x11 pixel font for labels and numeric annotations
- **Image display** — render PNG, RGB, and RGBA images inline via Kitty protocol

## CLI Usage

termplt includes a command-line executable for rendering plots without writing Rust code. Data is provided inline or via files, with optional per-series styling.

```bash
# Simple line plot
termplt --data "(1,1),(2,4),(3,9),(4,16)"

# Plot from a CSV file
termplt --data_file data.csv

# Scatter plot (no connecting lines)
termplt --data_file data.csv --line_style None

# Multiple series with automatic color cycling
termplt --data_file sine.csv --data_file cosine.csv

# Custom styling
termplt --data_file data.csv --marker_style HollowCircle --marker_color Cyan --marker_size 4 \
  --line_color Cyan --line_thickness 1

# Line-only plot (no markers)
termplt --data_file data.csv --marker_style None --line_color Lime --line_thickness 1
```

### CLI Flags

| Flag | Description |
|---|---|
| `--data "(x,y),(x,y),..."` | Inline data points |
| `--data_file <path>` | Read x,y data from a file (CSV, TSV, or whitespace-delimited) |
| `--marker_style <style>` | `FilledCircle`, `HollowCircle`, `FilledSquare`, `HollowSquare`, `None` |
| `--marker_color <color>` | Named color (e.g. `Blue`, `DARK_RED`, `lime`) |
| `--marker_size <pixels>` | Marker radius in pixels (default: 2) |
| `--line_style <style>` | `Solid` (default) or `None` (scatter plot) |
| `--line_color <color>` | Named color for connecting line |
| `--line_thickness <pixels>` | Line thickness in pixels (default: 0) |
| `--help` | Show usage help |
| `--help colors` | List all available color names |
| `--help markers` | List all available marker styles |

Style flags apply to the immediately preceding `--data` or `--data_file`. Repeat data flags for multiple series — each gets independent styling with automatic color/marker cycling when styles are not specified.

Data files support CSV headers (auto-detected and skipped), `#` comment lines, and blank lines.

## Requirements

- A Kitty-compatible terminal (Kitty, WezTerm, or any terminal supporting the [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/))
- Rust 2024 edition

## Quick Start

Add `termplt` to your `Cargo.toml`:

```toml
[dependencies]
termplt = { git = "https://github.com/EdCarney/termplt.git" }
```

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
cargo test        # Run all unit tests
cargo clippy      # Lint
cargo run -- --data_file data.csv   # Render a plot (requires Kitty-compatible terminal)
```

### Test Data Generation

A Python script is included to generate sample data files for validating CLI behavior:

```bash
python3 scripts/generate_test_data.py
```

This creates 13 data files in `test_data/` covering sine/cosine, polynomials, exponentials, parametric curves (circle, Lissajous), random scatter, Gaussian clusters, and more. The script prints example `cargo run` commands for each dataset.

## License

This project does not currently specify a license. All rights reserved by the author.
