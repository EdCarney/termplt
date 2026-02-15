# termplt Unit Test Memory

## Project Overview
- Rust terminal plotting library using Kitty graphics protocol
- Edition 2024, deps: crossterm, image, libc, nix, rgb
- Custom `Result<T>` type alias: `std::result::Result<T, Box<dyn Error>>`

## Test Framework
- Standard Rust `#[cfg(test)]` with `#[test]` attributes
- No external test framework (no mockall, no proptest)
- Tests co-located in modules via `mod tests {}` blocks
- Uses `#[should_panic]` and `#[should_panic(expected = "...")]` for expected panics

## Current Tests (61 total, all passing — as of 2026-02-15)
- `point.rs`: 15 tests (arithmetic ops, PointCollection limits, scale_to zero-span)
- `limits.rs`: 8 tests (invalid ranges, update, contains)
- `line.rs`: 10 tests (Bresenham all orientations, thickness, dashed todo)
- `text.rs`: 9 tests (num_to_str formatting, Label limits/mask centered/left-aligned)
- `graph.rs`: 5 tests (empty graph, series limits)
- `series.rs`: 5 tests (arithmetic ops, empty series panic)
- `canvas.rs`: 2 tests (empty canvas panic, single series draw)
- `encoding.rs`: 3 tests (1/2/3 byte base64 encoding)
- `window_ctrl.rs`: 4 tests (normal values, zero rows/cols/both)

## Modules With NO Tests (zero coverage)
- `plotting/common.rs` — safe_f64_to_u32, safe_f64_to_i32, conversion traits
- `plotting/graph_limits.rs` — XOnly/YOnly/XY shift_by and scale_to logic
- `plotting/line_positioning.rs` — limits(), convert_to, scale_to, shift_by
- `plotting/axes.rs` — get_labels, get_mask (XOnly/YOnly/XY)
- `plotting/grid_lines.rs` — get_mask (XOnly/YOnly/XY)
- `plotting/marker.rs` — all four marker styles get_mask
- `plotting/numbers.rs` — get_bitmap, scaling, padding
- `plotting/colors.rs` — constants only, no tests needed
- `kitty_graphics/ctrl_seq.rs` — CtrlSeq for all enum variants
- `kitty_graphics/rgb_imgs.rs`, `png_imgs.rs`, `rgba_imgs.rs` — I/O, hard to unit test
- `terminal_commands/` — all files; require real terminal I/O

## Known Bugs / Issues (from IMPROVEMENTS.md, updated)
- Division-by-zero: `window_ctrl.rs:51-52` (x_pix/cols when pix=0, cols!=0) — no test
- `GraphLimits::scale_to` divides by old_span without zero check (graph_limits.rs:49-50)
- `LineStyle::Dashed` contains `todo!()` (line.rs:220) — documented panic test exists
- `Label::limits()` and `Label::get_mask()` panic on `LeftAligned` — documented tests exist
- `CanvasBuffer::new()` panics on `TopBottom`/`LeftRight`/`TopBottomLeftRight` (canvas.rs:38)
- `.unwrap()` inside `Result`-returning fns: canvas.rs:143,160; graph.rs:202,236
- Base64 encoding missing `=` padding (encoding.rs) — no test documents this
- `numbers::get_bitmap()` panics on unsupported characters (numbers.rs:204)
- `images.rs:55,119` — panics on unsupported format/transmission combos

## Fixed Since Last Analysis
- `Point::scale_to` zero-span: now maps to midpoint (point.rs:117-127) — tests added
- Line drawing: Bresenham's algorithm replaces slope/intercept (line.rs) — 10 tests added
- `window_ctrl.rs`: zero rows/cols now returns `Err` — 3 tests added
- `safe_f64_to_u32/i32`: replaced unsafe `to_int_unchecked` with `clamp` (common.rs)
- `text.rs`: Label tests for Centered limits/mask and LeftAligned panic added

## File Paths
- Source: `/Users/edcarney/projects/termplt/src/`
- Plotting: `src/plotting/` (main logic)
- Kitty: `src/kitty_graphics/` (protocol encoding)
- Terminal: `src/terminal_commands/` (I/O, hard to unit test)
- Memory: `/Users/edcarney/projects/termplt/.claude/agent-memory/unit-test/`

## Notes
- Agent threads always reset cwd; use absolute file paths only
- No emojis in responses
- No colon before tool calls in prose
