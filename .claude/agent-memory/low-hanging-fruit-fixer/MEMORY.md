# Agent Memory: Low-Hanging Fruit Fixer

CLAUDE.md is the source of truth for commands, architecture and conventions; IMPROVEMENTS.md
tracks the improvement plan and what is still open. Paths below are relative to the repo root.

## Working rules
- Run `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` before
  committing; CI also builds docs with `-D warnings` (`#![warn(missing_docs)]`) and tests on the
  MSRV (1.88).
- The library returns `termplt::Result` (typed `Error` enum in `src/error.rs`); no `unwrap()`,
  `println!` or panics on user input in library code.
- `Series::draw_into` (fast path) must produce exactly the pixels of `Drawable::get_mask`; change
  both together (a test compares them).
- Rendering changes: regenerate golden snapshots with `TERMPLT_UPDATE_SNAPSHOTS=1 cargo test
  --test golden` and inspect them.
- Keep internal modules private; re-export what users need.

## Where things are
- `src/plot.rs`: `Plot` builder (what the CLI uses).
- `src/plotting/`: series, graph, canvas (layout + rendering), ticks, line/marker drawing, text.
- `src/terminal.rs`: `Terminal` (support query, size, tmux); `src/terminal_commands/`: protocol
  and terminal I/O.
- `src/bin/termplt/`: CLI (clap).

## Still open (see IMPROVEMENTS.md)
- Item 13: no text font beyond digits, so no titles, axis names or legends.
- Segment clipping at explicit limits (points outside are dropped instead).
