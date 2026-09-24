# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

```bash
cargo build                          # Build
cargo test                           # Run all tests (unit tests, co-located in modules)
cargo test plotting::graph_limits    # Run tests for a specific module
cargo test scale_to_with_zero_x_span # Run a single test by name
cargo clippy --all-targets -- -D warnings  # Lint (CI fails on any warning)
cargo fmt --check                    # Formatting (enforced in CI)
cargo run -- --data "(1,1),(2,4)"    # Render a plot via the CLI (needs a Kitty-protocol terminal)
```

CI (`.github/workflows/ci.yml`): build + test on Linux and Windows, clippy, rustfmt. No feature flags, no custom build scripts. Edition 2024 (let-chains are used, so Rust >= 1.88).

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

**Zero-span safety:** `Graph::view_limits()` pads zero-width dimensions (5% of the value, or ±0.5 around zero) before scaling, so single points and constant series are centered. As a fallback, `Point::scale_to` and `GraphLimits::scale_to` map a zero old span to `new_span / 2` (relative to the new origin, like the regular branch; callers shift afterwards).

### Rendering Pipeline (`canvas.rs` → `graph.rs`)

```
TerminalCanvas::draw()
  ├── get_drawable_limits()          # canvas area minus buffers/marker/axes thickness
  ├── graph.view_limits()            # finite data limits + GraphLimits overrides, clipped, padded
  ├── graph.scale_with_view(..)      # clip → shift-to-origin → proportional scale → shift-to-canvas
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

After rendering, the canvas bytes are sent via Kitty APC sequences: `encoding.rs` does custom RFC 4648 base64 (with padding), `kitty_cmds.rs` chunks to 4096-byte payloads, `ctrl_seq.rs` provides protocol key=value formatting. `TermCommand` writes commands to stdout; `execute_with_response` writes queries to `/dev/tty` (stdin/stdout on Windows) followed by a DA1 sentinel, reads the reply with a timeout under an RAII raw-mode guard, and fails fast with `TerminalCommandError::Unsupported` when the DA1 reply arrives first.

### Line Drawing (`line.rs`)

`BetweenPoints` lines use Bresenham's algorithm. `Horizontal`/`Vertical` lines use range iteration. Thickness is applied by shifting parallel copies (flat lines only). `LineStyle::Dashed` filters the ordered path with a 6-on/4-off pattern. Thickness is not yet applied to `BetweenPoints` lines (so series line thickness has no effect).

### Text/Number Rendering (`text.rs`, `numbers.rs`)

Bitmap font: 10x11 pixel grids for `0-9`, `.`, `-`, `e`, ` `; other characters render as a placeholder box. Supports scaling (pixel replication) and padding. `num_to_str` uses decimal when `0.1^sig_figs < |x| < 10^sig_figs`, otherwise scientific notation, with trailing zero stripping.

## Known Issues

See `IMPROVEMENTS.md` for the full prioritized list and status. Key open items:
- Tick labels are evenly spaced raw values (not "nice" numbers) and overlap or clip at small sizes
- Series line thickness is ignored for `BetweenPoints` lines
- Clipping to explicit limits drops points rather than clipping line segments
- `Limits::new` panics on inverted bounds (internal invariant); use `Limits::try_new` for untrusted input
- `Graph::shift_by` doesn't shift `grid_lines`
- No crate-level error type (uses `Box<dyn Error>` everywhere)
- No integration tests; all tests are unit tests co-located in source files
