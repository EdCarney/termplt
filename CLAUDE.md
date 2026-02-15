# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
cargo build                          # Build
cargo test                           # Run all tests (83 unit tests, co-located in modules)
cargo test plotting::graph_limits    # Run tests for a specific module
cargo test scale_to_with_zero_x_span # Run a single test by name
cargo clippy                         # Lint (warnings present but non-blocking)
cargo run                            # Render sample graphs to terminal via Kitty protocol
```

No CI, no feature flags, no custom build scripts. Edition 2024.

## Architecture

**termplt** is a Rust library for rendering 2D plots directly in Kitty-compatible terminals using the Kitty graphics protocol. User data flows through a generic type system, gets scaled to pixel coordinates, rendered to an in-memory canvas, then transmitted as RGB pixel data via Kitty APC escape sequences.

### Core Trait Hierarchy (`plotting/common.rs`)

`Graphable` is a blanket trait auto-implemented for any numeric type (`i32`, `u32`, `f32`, `f64`) satisfying arithmetic + comparison + `Into<f64>` + `Copy`. All data structures (`Point<T>`, `Limits<T>`, `Series<T>`, `Graph<T>`) are generic over `T: Graphable`.

Type conversion uses a layered system:
- `Convertable<U>` — base trait, converts via `fn(f64) -> U` function pointer
- `UIntConvertable` / `IntConvertable` / `FloatConvertable` — specialized with safe clamped casts (`v.clamp(0.0, u32::MAX as f64) as u32`)
- Implemented for scalars, `Point`, `Limits`, `Series`, `Graph`, `GraphLimits`, `Line`, `LinePositioning` — so the entire scene graph can be type-converted in one call

Coordinate transforms use two traits:
- `Scalable<T, U>` — proportional mapping between two `Limits` (old → new coordinate space)
- `Shiftable<T>` — translation by a `Point<T>` offset

Both are implemented recursively on composite types (Graph shifts all its Series, each Series shifts all its Points).

**Zero-span safety:** When `old_span == 0.0` in any dimension, `Point::scale_to` and `GraphLimits::scale_to` map to the midpoint of the new range instead of dividing by zero.

### Rendering Pipeline (`canvas.rs` → `graph.rs`)

```
TerminalCanvas::draw()
  ├── get_drawable_limits()          # canvas area minus buffers/marker/axes thickness
  ├── graph.limits()                 # data-derived limits + GraphLimits overrides
  ├── graph.scale(canvas_limits)     # shift-to-origin → proportional scale → shift-to-canvas
  ├── scaled_graph.get_mask()        # Drawable trait: returns Vec<MaskPoints> (colored pixel sets)
  │     ├── axes.get_mask()          # axis lines
  │     ├── grid_lines.get_mask()    # grid lines
  │     └── series.get_mask()        # markers + connecting lines per series
  ├── Canvas::set_pixels()           # write RGB8 into 2D pixel buffer
  └── labels → get_mask → set_pixels # axis tick labels (bitmap font)
```

The `Drawable` trait (`fn get_mask(&self) -> Result<Vec<MaskPoints>>`) is implemented by `Series`, `Line`, `Marker`, `Label`, and `Graph`. Each returns pixel coordinates + colors; the canvas composites them.

### GraphLimits State Machine (`graph.rs`, `graph_limits.rs`)

`Graph` manages `Option<GraphLimits<T>>` with transitions: `None` → `XOnly`/`YOnly` → `XY`. Calling `with_x_limits` on a `YOnly` graph produces `XY`; calling it on `XY` updates only x. `Graph::limits()` merges these overrides with data-derived limits. Explicit limits also trigger point clipping during `Graph::scale`.

### Kitty Protocol (`kitty_graphics/`)

After rendering, the canvas bytes are sent via Kitty APC sequences: `encoding.rs` does custom base64 (no padding — known bug), `kitty_cmds.rs` chunks to 4096-byte payloads, `ctrl_seq.rs` provides protocol key=value formatting. `TermCommand` trait handles stdout writing and optional raw-mode response reading.

### Line Drawing (`line.rs`)

`BetweenPoints` lines use Bresenham's algorithm. `Horizontal`/`Vertical` lines use range iteration. Thickness is applied by shifting parallel copies (flat lines only). `LineStyle::Dashed` is `todo!()`.

### Text/Number Rendering (`text.rs`, `numbers.rs`)

Bitmap font: 10x11 pixel grids for `0-9`, `.`, `-`, `e`, ` `. Supports scaling (pixel replication) and padding. `num_to_str` uses decimal when `0.1^sig_figs < |x| < 10^sig_figs`, otherwise scientific notation, with trailing zero stripping.

## Known Issues

See `IMPROVEMENTS.md` for the full prioritized list. Key items:
- Public API panics instead of returning `Result` in many places (`Series::new(&[])`, `Limits::new` with inverted bounds, `TextStyle::new(scale < 1)`)
- `.unwrap()` inside `Result`-returning functions in `canvas.rs` and `graph.rs` defeats error propagation
- `TextPositioning::LeftAligned`, `BufferType::TopBottom/LeftRight/TopBottomLeftRight` panic with "Not implemented"
- `println!` in `canvas.rs` and `graph.rs` corrupts Kitty escape sequence output
- `Graph::shift_by` doesn't shift `grid_lines`
- No crate-level error type (uses `Box<dyn Error>` everywhere)
- No integration tests; all tests are unit tests co-located in source files
