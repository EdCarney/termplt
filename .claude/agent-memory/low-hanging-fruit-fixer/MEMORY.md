# Agent Memory: Low-Hanging Fruit Fixer

## Codebase Structure
- All source under `/Users/edcarney/projects/termplt/src/`
- `plotting/` contains the core rendering pipeline: canvas.rs, graph.rs, series.rs, line.rs, text.rs, etc.
- `kitty_graphics/` handles Kitty protocol encoding
- `terminal_commands/` handles terminal I/O (images, CSI commands)
- `window_ctrl.rs` handles terminal window size detection (nix-only)
- Tests are co-located in each module under `#[cfg(test)] mod tests`

## Test Patterns
- Baseline: 88 lib tests + 34 binary tests (as of 2026-02-15 before fixes)
- Use `use super::*` in test modules; for cross-module types use `use crate::plotting::module::Type`
- `#[should_panic]` tests exist for intentional panics
- No integration test directory; all tests are unit tests

## Module Risk Assessment
- **High risk**: `plotting/common.rs` (trait hierarchy), any trait changes ripple everywhere
- **Medium risk**: `graph.rs` (scale/shift pipeline), `canvas.rs` (rendering entry point)
- **Low risk**: `encoding.rs`, `text.rs`, `numbers.rs`, `colors.rs` -- isolated modules

## Resolved Issues
- #13: Removed println! from canvas.rs and graph.rs (PR #23)
- #15: Replaced unwrap() with ? in Result-returning fns in canvas.rs and graph.rs (PR #24)
- #16 (partial): Implemented BufferType::TopBottom/LeftRight/TopBottomLeftRight (PR #25)
  - TextPositioning::LeftAligned still unimplemented (needs rendering logic)

## Key Observations
- `Axes::new()` requires explicit args (no default); use `Axes::new(AxesPositioning::XY(LineStyle::default()), TextStyle::default())`
- `TextStyle::default()` and `LineStyle::default()` exist as const/static methods
- `Graph::limits()` returns `Option<Limits<T>>` -- None when no data
- Pre-existing clippy warnings (5): unused imports in graph.rs/marker.rs, unused var in line.rs, dead code in images.rs
