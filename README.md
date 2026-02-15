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
cargo build       # Build the library
cargo test        # Run all unit tests
cargo clippy      # Lint
cargo run         # Render sample graphs (requires Kitty-compatible terminal)
```

## License

This project does not currently specify a license. All rights reserved by the author.
