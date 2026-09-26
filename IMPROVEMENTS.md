# termplt Improvement Plan

Review of commit `74918e3` (v0.1.2). How the review was done:

- Read every file under `src/`, plus the workflows, README and CLAUDE.md.
- Ran `cargo test` (126 tests pass) and `cargo clippy --all-targets` (about 59 warnings).
- Ran the CLI with stdin/stdout not attached to a terminal, and inside a pty whose terminal never answers queries.
- Wrote throwaway library tests to probe edge cases.
- Rendered plots to PNG and inspected them.

Status (2026-09-24): Phase 1 (items 1–6, 8, 9, and part of 42), Phase 2 (items 7, 10–12, 14, 15, 23 and 43), Phase 3 (items 16, 17 and 19–24, plus most of 18) Phase 4 (items 25–31; Unicode fallback renderer and tmux Unicode placeholders deferred) Phase 5 (items 32–41, with breaking changes for 0.2.0), Phase 6 (items 42 and 44–47) and Phase 7 (fixes from a review of Phases 1–5, below) are done. See the note under each item.

Items marked **(reproduced)** were confirmed by running code. The rest come from reading the source.

Priority: **P0** hang, crash or wrong output · **P1** big usability or quality gain · **P2** polish and maintainability.

---

## P0: hangs, crashes, wrong output

### 1. The CLI hangs forever when the terminal doesn't answer a CSI query (reproduced)
✅ **Done (Phase 1).** `poll(2)` on `/dev/tty` (reader thread on Windows), DA1 sentinel, RAII raw-mode guard, 2 s default timeout. Unix path checked in a pty; the Windows path is compile-checked only.

- `responses.rs:48-65`: `stdin.read_exact` blocks. The 1 s timeout is only checked *between* bytes, so a terminal that never replies blocks forever. Repro: `script -qc "termplt --data '(1,1),(2,2)'" /dev/null` never returns. This affects tmux without passthrough, screen, the VS Code terminal and any xterm that doesn't answer `CSI 14 t`.
- The query is written *before* raw mode is enabled (`responses.rs:48-49`), so the reply can be echoed to the screen.
- Nothing guarantees `disable_raw_mode`. A `?` early return or a panic (item 2) leaves the user's shell in raw mode.
- **Fix:**
  - Use an RAII guard that enables raw mode *before* writing and restores it in `Drop`.
  - Use a real timeout: `poll(2)` on Unix, `WaitForSingleObject` on Windows, or a reader thread plus `recv_timeout`.
  - Better still, add a sentinel. Append the DA1 query (`CSI c`) after every query. Every VT-compatible terminal answers DA1, so if the DA1 reply arrives first, the query is unsupported and no timeout is needed. The kitty docs recommend this pattern for graphics detection [1].
  - Read replies from `/dev/tty` instead of stdin (on Windows, `CONIN$`). This also unblocks piping data into the CLI (item 16).
- **Trade-off:** a reader thread is portable, but a blocking read can't be cancelled, so the thread leaks until a byte arrives. `poll` is clean but needs a `cfg` split per OS. The DA1 sentinel removes most of the dependence on timeouts.

### 2. Panics on unexpected terminal replies
✅ **Done (Phase 1).**

`csi_cmds.rs:48-57, 80-87, 98-105` use `expect`/`assert_eq!` to parse replies. A malformed or interleaved reply (for example, the user presses a key during the query) panics while raw mode may still be on. Return `Err` instead.

### 3. Non-TTY use gives a cryptic error and writes escape bytes into the pipe (reproduced)
✅ **Done (Phase 1).** Queries now go to `/dev/tty`, and the CLI checks `IsTerminal` on stdout up front; `--output` followed in Phase 3.

`termplt --data ... </dev/null` prints `\x1b[14t` to stdout, then `Error: No such device or address (os error 6)`. Check `std::io::IsTerminal` up front and print an actionable message, for example "stdout is not a terminal; use `--output plot.png`" (item 18).

### 4. Degenerate data (one point, or a constant x or y) is drawn off-center, with no axes or grid (reproduced)
✅ **Done (Phase 1).** `Graph::view_limits()` pads zero-width dimensions; the scale fallback is now relative.

- `Point::scale_to` (`point.rs:117`) returns the *absolute* midpoint of the new range. `Graph::scale_to` then shifts by the new minimum again (`graph.rs:260,270`), so the offset is applied twice. On a 101×101 canvas with buffer 10, a single point lands at pixel (60,40) instead of (50,50). A constant-y series is drawn at 40% height instead of 50%. `graph_limits.rs:62` has the same bug.
- With zero span the axes and grid lines have zero length and don't render. All the tick labels stack on top of each other.
- **Fix:** pad degenerate limits before scaling (for example ±0.5, or ±5% of |v|), which is what matplotlib does. The zero-span branch then becomes unreachable, but keep it and make it relative (`new_span / 2`).

### 5. Explicit axis limits that exclude every point of a series panic (reproduced)
✅ **Done (Phase 1)** for the panics. Segment clipping (Liang–Barsky) is still open.

- `Graph::scale` filters out points outside the limits (`graph.rs:183-192`). An emptied series then hits `0..self.data.len() - 1` in `series.rs:94`, which underflows: `attempt to subtract with overflow` in debug builds, and an out-of-bounds index in release builds.
- If *all* points are excluded, `graph.rs:200` panics via `expect`.
- **Fix:** iterate with `data.windows(2)`, skip empty series, and return `Err` when nothing is left to draw.
- **Related:** dropping out-of-range *points* also removes line segments that cross the boundary, so lines stop short of the plot edge. Clip *segments* to the limit rectangle with Liang–Barsky [2] instead.

### 6. NaN or ±∞ in the data crashes rendering (reproduced)
✅ **Done (Phase 1).** The library ignores non-finite points, the CLI skips them with a warning, and unknown glyphs render as a box.

- The CLI accepts `nan` and `inf`, because `f64::from_str` does.
- A NaN as the first point poisons the limits: the fold in `point.rs:18-37` compares with `<`, which is always false against NaN.
- An infinite or NaN value reaches tick labels as `"inf"`/`"NaN"`, and `numbers.rs:204` panics with "Bitmap not defined for character".
- **Fix:** reject or filter non-finite values at the library boundary; the CLI should skip them and warn. `get_bitmap` should fall back to a placeholder glyph instead of panicking.

### 7. `line_thickness` has no effect on series lines (reproduced)
✅ **Done (Phase 2).** A disc is stamped at each pixel (option A).

- Lines between points (`line.rs:207`) use Bresenham only. Thickness is applied only to horizontal and vertical lines (axes and grid). Measured: thickness 0 and thickness 3 both produce exactly 100 red pixels. The README advertises `--line_thickness` and uses it in an example.
- **Fix, option A:** stamp a disc of radius *t* at each Bresenham pixel. It's simple and gives round joins, at O(n·t²).
- **Fix, option B:** draw offset parallel lines or fill a polygon per segment. This is cheaper, but joins need extra work to avoid gaps.

### 8. Other panics reachable through the public API
✅ **Done (Phase 1)**, except `Limits::new`, which is kept as a documented invariant panic. `Limits::try_new` was added for untrusted input.

Each of these crashes the program instead of returning an error:
- `Limits::new` with inverted bounds (`limits.rs:42`), reachable with `graph.with_x_limits(2.0, 0.0)` (reproduced).
- `TerminalCanvas::new(0, h)` underflows on `width - 1` (`canvas.rs:72`) (reproduced).
- `with_graph` on an empty graph (`canvas.rs:138`).
- `Series::new(&[])` (`series.rs:37`).
- `LineStyle::Dashed` is a `todo!()` (`line.rs:220`) (reproduced).
- `TextPositioning::LeftAligned` (`text.rs:262,285`).
- `Image::new`/`display` with `TempFile`/`SharedMemory` (`images.rs:55,119`).
- `PositioningType::Centered` underflows when the image is bigger than the window (`images.rs:106`).
- `.unwrap()` inside `Result`-returning code: `series.rs:88`, `grid_lines.rs:31,38`, `text.rs:271-272,278`, `kitty_cmds.rs:30`.

**Fix:** make constructors that validate input return `Result`, or make invalid states unrepresentable (`NonZeroU32` for canvas size). Implement `Dashed` by skipping pixels along the Bresenham path with an on/off pattern, or remove the variant until it's implemented.

### 9. CSV error line numbers are off by one when a header is skipped (reproduced)
✅ **Done (Phase 1).**

`termplt.rs:352-356` consumes the header *before* `enumerate()`. The input `x,y\n1,2\nfoo,3` reports `bad.csv:2`, but `foo` is on line 3. Enumerate first, then skip.

---

## P1: plot quality (what the user sees)

### 10. Tick labels are unreadable at the CLI's default size (reproduced by rendering)
✅ **Done (Phase 2):** nice ticks (`plotting::ticks`), a tick count fitted to the pixel size, automatic label margins, an exact zero, and shared decimal places. Instead of loose labeling I used a 5% data margin with tight ticks, as matplotlib does by default.

At the CLI's default geometry for a typical window (500 px canvas, 50 px buffer):
- The x labels overlap into one unreadable string.
- The y labels are clipped at the left edge, so `-0.99` renders as `0.99`.
- Tick values are arbitrary (`0.733, 1.47, …`), and the zero line reads `3e-4`.

**Causes:**
- The raw data range is always split into exactly 10 divisions (`axes.rs:46`, `grid_lines.rs:25`).
- Glyphs are a fixed 10 px wide.
- The buffer is sized with no knowledge of label width.

**Fixes:**
- Pick "nice" tick steps from {1, 2, 5} × 10ᵏ (Heckbert [3]), with the same number of decimal places on every label of an axis.
- Derive the tick count from the available pixels divided by the widest label.
- Lay out margins automatically: compute the left and bottom margins from the actual label extents instead of a single uniform buffer.
- Snap values within ε·span of 0 to exactly 0.
- Optionally extend the axis limits out to the nearest tick ("loose" labeling [3]).

### 11. Data touches the axes
✅ **Done (Phase 2).** `Graph::view_limits` adds a 5% margin on axes without explicit limits.

Markers at the minimum and maximum sit on the axis lines. Add default padding of about 5% of the span.

### 12. Grid lines are drawn over the axes
✅ **Done (Phase 2).**

The mask order is axes, then grid, then series (`graph.rs` `get_mask`). Draw the grid first.

### 13. No title, axis names or legend
◐ **Titles and axis names done (0.3.0):** TrueType text (embedded Go font, `ab_glyph`), `Plot::title`/`x_label`/`y_label`, `--title`/`--xlabel`/`--ylabel`, and names from CSV headers. The legend is next.

The bitmap font covers only `0-9 . - e` and space. Add a small ASCII bitmap font (for example a public-domain 6×8 font, scaled) so titles, axis names and a legend with per-series names become possible. In the CLI that could be `--label "sin(x)"` per series and `--title`.

### 14. Default text color is black
✅ **Done (Phase 2).** A tick-label color that equals the background is swapped for black or white.

`TextStyle::default()` is black (`text.rs:96`), and the default canvas in the examples is also black, so labels are invisible. Pick a default that contrasts with the background.

### 15. The CLI's `--marker_style None` is a hack
✅ **Done (Phase 2).** Added `MarkerStyle::None`.

`termplt.rs:495` draws a zero-size *black* square at every point. That leaves a visible dot on non-black backgrounds and overwrites the grid. Support "no marker" in the library with `Option<MarkerStyle>` or a `MarkerStyle::None` variant.

---

## P1: CLI usability

### 16. Read data from stdin
✅ **Done (Phase 3).** `-` or piped stdin is read once and cached.

Piping (`some_cmd | termplt`) is the most common workflow in a terminal. Accept `-` or detect a non-TTY stdin. This requires terminal queries to go through `/dev/tty` rather than stdin (item 1).

### 17. Column selection
✅ **Done (Phase 3):** `-x/--x-col`, `-y/--y-col` (name, 1-based index, or `index`), comma-separated y columns, and single-column files plotted against the row number.

- Add `--x-col` and `--y-col`, by header name or index.
- Allow several y columns, producing one series per column from a single file.
- Plot single-column data as y against the row index.

### 18. Output and layout flags
◐ **Mostly done (Phase 3):** `--width`, `--height`, `-o/--output`, `--xlim`, `--ylim`, `--bg` and `--no-grid` are in. `--title` is done (0.3.0); `--log-x`/`--log-y` still need log scaling in the library and are deferred.

- `--width` and `--height` (or `--rows` and `--cols` in cells).
- `--output plot.png`: the `image` crate is already a dependency. This also covers non-kitty terminals, CI and reports.
- `--xlim` and `--ylim` (the library supports limits, but the CLI doesn't expose them).
- `--title`, `--bg`, `--no-grid`, `--log-x` and `--log-y`.

### 19. The inline data parser is too strict (reproduced)
✅ **Done (Phase 3).**

`"(1,1), (2,2)"` fails because `termplt.rs:307` splits on the literal `"),("`. Tokenize on parentheses instead, and also accept the `1,1 2,2` form.

### 20. Color names are too strict (reproduced)
✅ **Done (Phase 3).** Added `colors::from_name` normalization and `colors::parse` (hex).

`DarkRed` fails and only `DARK_RED` works (`colors.rs:286`). Normalize names by removing `_`, `-` and spaces before comparing. Also accept `#RRGGBB`.

### 21. Argument parser: missing `--version`, non-standard snake_case flags, no suggestions
✅ **Done (Phase 3):** clap, with grouped `-s/--series "file=...,color=..."` specs. Old flag spellings are kept as aliases, and the CLI sits behind the default `cli` feature.

The parser is hand-rolled.

| Option | Pros | Cons |
|---|---|---|
| `clap` (derive) [4] | Generated help, `--version`, shell completions, "did you mean" suggestions, kebab-case flags with snake_case kept as hidden aliases | More compile time and binary size. Tying style flags to the *preceding* `--data` needs `ArgMatches::indices_of`, which is awkward. |
| `lexopt` [5] | Tiny, no dependencies, keeps the current ordered per-series semantics naturally | Help text, completions and suggestions stay manual |
| Keep the current parser | No new dependency | About 130 lines of repetitive parsing code, and every new flag makes it longer |

**Recommendation:** `clap`, if you're willing to replace the per-series flag order with explicit grouping (for example `--series "file=a.csv,color=red"`). Otherwise `lexopt`.

### 22. Hard-coded canvas size
✅ **Done (Phase 3).** The default size is the terminal's full width by 60% of its height, at most 2:1.

`termplt.rs:583` always draws a square at half the smaller window dimension, which wastes most of the width in a wide terminal. Default to something like the full width × about 60% of the height, capped by the window, and let flags override it.

### 23. `--verbose` duplicates the layout math
✅ **Done (Phase 2).** `--verbose` now reports `get_drawable_limits()`.

`termplt.rs:593-617` re-implements `get_drawable_limits`, so the two can drift apart. Call `TerminalCanvas::get_drawable_limits()` instead.

### 24. Missing values abort the whole file
✅ **Done (Phase 3).** Empty, `NA`, `n/a`, `null`, `none`, `-` and `?` are skipped with a count.

An empty field or `NA` stops parsing. Offer a skip-and-warn mode, or at least a flag for it, and report how many rows were skipped.

---

## P1: terminal and protocol robustness

### 25. Detect graphics support before rendering
✅ **Done (Phase 4):** `kitty_cmds::query_support()` sends an `a=q` query plus DA1. The CLI stops with a hint to use `--output` when the terminal answers DA1 but not the query, and warns and draws anyway when it answers nothing. The braille/half-block fallback renderer is not done.

Send an `a=q` query with an image id, followed by DA1 [1]. If the terminal doesn't support the protocol, show a clear error or fall back. Fallback options:
- `--output png`.
- A Unicode braille or half-block renderer. Resolution is lower, but it works everywhere, including in tmux and CI logs.

### 26. Window-size lookup
✅ **Done (Phase 4).** `get_window_size()` uses `crossterm::terminal::window_size()` (`TIOCGWINSZ`) and queries only the missing parts (`CSI 14t` for pixels, `18t` for cells). If both fail, the CLI estimates the size from the cell count with a warning.

Commit `4926cc0` replaced the ioctl with CSI queries for Windows. On Unix, prefer `TIOCGWINSZ` (`crossterm::terminal::window_size()` returns pixel sizes). It needs no terminal round trip, so it can't hang. Fall back to `CSI 14t`/`18t`, then to a default size with a warning. Use `cfg` to keep both paths.

### 27. Compress the image data sent to the terminal (measured)
✅ **Done (Phase 4):** `Image::png_from_rgb` with `f=100`. Measured on a dense 1600×800 plot (release build): raw 3.84 MB; PNG with default compression and adaptive filtering 27 KB in 11 ms; the fast setting gives 68 KB in 3 ms. The CLI's typical plot drops from 2.16 MB to about 7 KB.

An 800×800 plot is 1.92 MB raw, or 2.56 MB after base64, sent on *every* render. The same 500 px plot saved as PNG is about 12.6 KB, versus 750 KB raw: roughly 60× smaller. Over SSH this is the difference between instant and multi-second.

Two ways to fix it:
- Send `f=100` (PNG), encoded with the existing `image` dependency.
- Keep `f=24` and add `o=z` (zlib) [1].

**Trade-off:** a few ms of CPU to encode, in exchange for 1–2 orders of magnitude less bandwidth. PNG is lossless.

### 28. tmux and screen
✅ **Done (Phase 4)** for tmux: `Passthrough::Tmux` (from `$TMUX`) wraps every chunk. The CLI checks `allow-passthrough` (via `tmux show-options -pAv`) and stops with the fix when it's off, then draws with `C=1` and moves the cursor down itself. Checked with tmux 3.4 inside a pty: the PNG reaches the outer terminal unchanged. Not done: Unicode placeholders (the image still vanishes on a tmux redraw) and GNU screen.

- When `$TMUX` is set, wrap the APC sequence in DCS passthrough (`\ePtmux;` plus doubled ESC, then `\e\\`). The user must also set `allow-passthrough on`.
- Kitty's Unicode-placeholder mode (`U=1`) [1] keeps the image in place when tmux redraws. Document the setup either way.

### 29. Suppress terminal replies
✅ **Done (Phase 4).** Display commands send `q=2`.

Add `q=2` to transmit commands. Without an image id the terminal doesn't reply today, but this becomes necessary once ids are used, for example to replace a plot in place.

### 30. File transmission
✅ **Done (Phase 4).** `Image::new` makes `File`/`TempFile` paths absolute (`std::path::absolute`, which doesn't need the file to exist). The `Transmission` docs note that the file media need the terminal on the same machine (not SSH).

`Transmission::File` sends the path unchanged, but kitty requires an absolute path. Canonicalize it first. Also document that file transmission doesn't work over SSH.

### 31. Windows is untested
✅ **Done (Phase 4)** as documentation. The README lists supported terminals and gives a three-step manual checklist for Windows. The window size uses the console API for cells. Still not verified interactively.

Reading VT replies from a raw console stdin depends on VT input mode. CI only runs unit tests on Windows, so this path is unverified. Add a manual test checklist, or state in the README which Windows terminals are supported (for example WezTerm).

---

## P2: library API ergonomics

### 32. Add a high-level entry point
✅ **Done (Phase 5):** `termplt::Plot` with `line`, `scatter`, `line_points` and `series`, plus limits, size, background and grid, and `show()`/`show_in()`, `save_png()` and `render()`. `show()` sizes itself to the terminal through the new `terminal::Terminal`, which is the CLI's support/tmux/size logic moved into the library. Also added `termplt::prelude`. `.title()` is done (0.3.0).

The README example needs about 50 lines, 10 imports and hand-wired `Image` + `PixelFormat` + `Transmission` code.
- Add `termplt::prelude`.
- Add a one-call API that sizes itself to the terminal like the CLI does, for example `Plot::new().line(&xs, &ys).scatter(&pts).title("…").show()?`, plus `.save_png(path)`.
- Keep the current builders as the lower layer.

### 33. Accept common input shapes
✅ **Done (Phase 5):** `Series::from_xy` and `Series::from_y`, `FromIterator<(x, y)>`, `From<Vec<(x, y)>>`, `From<&[(x, y)]>`, `From<(xs, ys)>` for slices, `Vec`s and `&Vec`s, and `From<Vec<Point<T>>>`, each with mixed numeric types.

`Series::from_xy(&xs, &ys)`, `impl FromIterator<(T, T)>`, and `From<Vec<(T, T)>>`.

### 34. Reconsider the generic numeric design
✅ **Done (Phase 5), option B at the API.** `Series`, `Graph` and `TerminalCanvas` store `f64`, and `Graphable` uses a crate `ToF64` trait (`as` casts), so every primitive integer works. `Point`/`Limits` stay generic for pixel math. `Scalable`/`Shiftable` and the `GraphLimits` state machine are gone (−640 lines).

- `Graphable` requires `Into<f64>`, so **`i64`, `u64` and `usize` are excluded**. Those are the most common types for counts and indices.
- The pipeline converts everything to `f64` before scaling anyway, so the `Convertable`/`Scalable`/`Shiftable` layers over `T` buy nothing at runtime.
- **Option A:** keep generics and switch the bound to `num_traits::AsPrimitive<f64>`/`ToPrimitive`.
- **Option B:** accept anything convertable at the API boundary and use `f64` internally. That removes a large share of the trait code in `common.rs` and the per-type impls.
- **Trade-off:** option B is a breaking change, but it greatly simplifies maintenance. Option A is additive.

### 35. Typed errors
✅ **Done (Phase 5):** `termplt::Error` (`#[non_exhaustive]`, `Send + Sync`, hand-written `Display`/`source`, no `thiserror` dependency) replaces `Box<dyn Error>`. The CLI matches on variants.

Replace `Box<dyn Error>` with `pub enum Error` (using `thiserror`), so callers can distinguish cases like `TerminalUnsupported`, `InvalidData` and `CanvasTooSmall`. Two current error types are unhelpful: `ImageError`'s `Display` just prints its `Debug` output, and `TerminalCommandError` carries no context.

### 36. Implement the standard traits
✅ **Done (Phase 5).** `Default` for `LineStyle`, `Graph`, `Series` and `Plot`, plus `LineStyle::solid`/`dashed`. `Debug`/`Clone`/`Copy`/`PartialEq` derives on `BufferType`, `PositioningType`, `Image`, `WindowSize` and the protocol enums. `MarkerStyle` was already `Copy`.

Follow C-COMMON-TRAITS [6]:
- Implement `Default` instead of inherent `default()` functions (clippy `should_implement_trait`).
- Derive `Copy` for `MarkerStyle`.
- Derive `Debug` and `Clone` for `BufferType`, `PositioningType` and the `ctrl_seq` enums.

### 37. Public surface is too wide
✅ **Done (Phase 5).** `kitty_graphics`, `terminal_commands` and `plotting::{common, numbers, ticks}` are private, as are `Marker`, `Line`, `TextChar` and the point helpers. What users need is re-exported from `termplt::terminal` and `termplt::plotting`. The png/rgb/rgba demo helpers and dead protocol code were removed.

`numbers`, `encoding`, `ctrl_seq`, `csi_cmds` and `kitty_cmds` are public, so any change to them is a semver break. Make them `pub(crate)` or `#[doc(hidden)]` before 1.0.

### 38. Rustdoc
✅ **Done (Phase 5).** `#![warn(missing_docs)]` and `#![doc = include_str!("../README.md")]`, so the README examples are doctests. CI runs `cargo doc` with `-D warnings`.

Most public items have no docs.
- Add crate-level docs.
- Add `#![warn(missing_docs)]`.
- Use `#![doc = include_str!("../README.md")]` so the README example is compiled as a doctest.

### 39. `Limits::intersects` misses some overlaps
✅ **Done (Phase 5):** the interval-overlap test, with a cross-shaped case.

`limits.rs:84` only checks whether a corner of one rectangle lies inside the other, so it misses cross-shaped overlaps. It's used for label collision. Use an interval-overlap test on each axis.

### 40. Performance (measured)
✅ **Done (Phase 5), pixel-exact.** 1M points at 800×800 (release): scatter 1145 → ~115 ms, line 412 → ~210 ms, line + markers 1562 → ~270 ms, thick line 7395 → ~540 ms. Uses a flat buffer, `Series::draw_into` with marker stamps, incremental disc stamps and repeated-pixel skipping, and a fold for `Graph::limits`. Decimation was not needed, and it would have changed pixels.

1M points on an 800×800 canvas take 1.37 s in a release build.
- Each marker allocates a `Vec<Point<u32>>`.
- The filled circle regenerates overlapping pixel ranges.
- The canvas is a `Vec<Vec<RGB8>>` that `get_bytes` copies into another buffer.

Improvements:
- Write directly into one flat `Vec<u8>`.
- Precompute a marker "stamp" once per style.
- Decimate dense line series (min and max per pixel column).

### 41. Trim dependencies
✅ **Done (Phase 5).** `image` now has only `png` (unique dependencies 125 → 63). The CLI's `--output` accepts only `.png`.

`image` with default features pulls in many codecs. Use `default-features = false, features = ["png"]`.

---

## P2: testing, CI, release, docs

### 42. CI gaps
✅ **Done (Phases 1, 5 and 6):** rustfmt and `clippy --all-targets -D warnings` (Phase 1), `cargo doc -D warnings` (Phase 5), macOS in the test matrix, and `rust-version = "1.88"` with an MSRV job running `cargo test --locked` on 1.88 (Phase 6). 1.88 is the real minimum: let-chains need it, and the locked dependencies build on it.

- No `cargo fmt --check`.
- `cargo clippy` runs without `--all-targets -- -D warnings`. There are about 59 warnings today, most of them fixable with `cargo clippy --fix`.
- macOS binaries are released but never tested.
- No `cargo doc` step with `RUSTDOCFLAGS=-D warnings`.
- No MSRV: set `rust-version = "1.85"` in `Cargo.toml` (edition 2024 needs at least 1.85) and add a CI job for it.

### 43. Golden-image and property tests
✅ **Done (Phase 2):** `tests/golden.rs` (6 snapshot scenes) and `tests/properties.rs` (proptest). The property tests found and fixed an overflow when scaling tiny spans.

- Render to PNG and compare against checked-in snapshots. That would have caught items 4, 7 and 10.
- Add `proptest` tests for scaling and limits, with the invariant "every scaled point lies inside the drawable limits", using arbitrary finite and non-finite input.

### 44. Make terminal I/O testable
✅ **Done (Phases 1, 4 and 6).** Reply parsing and timeouts are unit-tested through a `ByteSource` fake (Phase 1) and the setup decisions through a `Backend` fake (Phase 4). Phase 6 adds `tests/pty.rs`: the CLI runs on a pty against a scripted fake terminal (9 scenarios, including tmux via a fake `tmux` script), and the tests decode the transmitted PNG. Each assertion was checked by breaking the code it covers (ESC doubling, `q=2`, the newline after the image, the DA1 short-circuit, tmux row count, skipping the query in tmux). `tests/cli.rs` covers the piped cases on every OS.

`TermCommand` hard-codes stdout and stdin. Inject `Read + Write` so reply parsing, timeouts and chunking can be unit-tested against a fake terminal.

### 45. Test temp files can collide
✅ **Done (Phase 6).** The CLI's parsing tests had already moved to in-memory data; the remaining `save_png` test and the new end-to-end tests use `tempfile`.

The CLI tests write fixed file names into the shared temp directory, so concurrent runs can collide. Use `tempfile`.

### 46. Release workflow gaps
✅ **Done (Phase 6)** without extra tooling: runs on a pushed `v*` tag (or by hand), checks the tag against `Cargo.toml`, runs the tests and `cargo publish --dry-run`, tests on each target before building, adds the version to the archive names, attaches `SHA256SUMS`, marks `-rc` style tags as pre-releases, and runs `cargo publish` only when a `CARGO_REGISTRY_TOKEN` secret exists. Manual runs now tag the commit that was built.

- No tests before building.
- No check that the tag matches the `Cargo.toml` version.
- No `cargo publish`.
- No checksums (`SHA256SUMS`).

Consider `cargo-dist` or `cargo-release`.

### 47. Docs are out of date
✅ **Done (Phases 5 and 6).** README install snippet, CLAUDE.md and the test counts were updated in Phase 5. Phase 6 adds `CHANGELOG.md`, points the README's `test_data/` examples at the generator script, documents the test and release setup, and rewrites the `.claude/agent-memory` notes without absolute paths or stale bug lists.

- The README's install snippet says `termplt = "0.1.0"`, but the crate is at 0.1.2.
- README examples use files in `test_data/`, which isn't committed (it's generated by a Python script). Commit a few small samples, or make the examples self-contained.
- CLAUDE.md is out of date:
  - It says there's no CI.
  - It lists as open several issues that are already fixed: missing base64 padding (fixed in `4502419`), `println!` and `unwrap()` in the library, and the `BufferType` panics (fixed in #23–#25).
  - It gives the test count as 83; there are now 126.
- `.claude/agent-memory/*` contain absolute macOS paths and outdated bug lists.
- Add a `CHANGELOG.md`.

---

## Phase 7: review of Phases 1–5

A review of `3f59d09` (after Phase 5) re-ran every check, probed edge cases with the CLI and a scratch crate, and found no regressions but 15 issues. All are fixed except where noted.

- **R1 Tick labels with a large offset (reproduced).** Timestamps got six `1.700000e9` labels, and the right end label overlapped its neighbour. Plain decimals are used when shorter than scientific, the mantissa may use 15 digits, `fit_ticks` rejects repeated labels, and the fit checks use the clamped label positions. *Fixed in 0.2.1:* near 1e15, where f64 can't represent the steps, labels had wrong values at uneven spacing. Axes whose range shares at least 4 leading digits now get matplotlib's offset (`+1e15`, `+1.7e9`), and their ticks are fitted and placed relative to it. The offset label only needed a bitmap `+`, not letters.
- **R2 Lines bridged removed points (reproduced).** `--ylim 0,10` on a peak drew a line along the bottom. `Graph::visible` leaves a NaN gap where points were removed, and drawing skips it.
- **R3 `GridLines::XOnly`/`YOnly` docs were backwards.** Docs now match the (unchanged) behaviour; a test pins it.
- **R4 Layout overflow (reproduced).** Saturating arithmetic, CLI limits of 0..=100 px, `TextStyle` scale/padding clamps, and property tests that reach `u32::MAX`.
- **R5 Windows console.** VT input is enabled while a query runs. *Left:* the reader thread keeps reading the console after a query, which can swallow a typed line in a long-running interactive program; documented. Compiled and linted for Windows, not run there.
- **R6 Misleading timeout text and size errors.** Neutral messages; a terminal with no size at all is assumed to be 80x24 cells; size errors get a `--width/--height` hint.
- **R7 CSV header detection.** A BOM is skipped and `;` is a delimiter. *Skipped:* warning about numeric-looking headers.
- **R8 Single column with `-y 1`** is plotted against the row number.
- **R9 Error messages repeated by reporters.** `source()` forwards the wrapped error's source.
- **R10 Division by zero** for terminals reporting fewer pixels than cells: rejected as `InvalidWindowSize`.
- **R11 `Label` overflow near `u32::MAX`, and `Text::from_number(-0.5, 1)` giving `-0`.** Saturating placement; `num_to_str` rewritten to round.
- **R12 Stale docs** in `TextStyle::with_color`, CLAUDE.md and this file.
- **R13 Input shapes** `[(x, y); N]`, `&[(x, y); N]`, `&Vec<(x, y)>`, `([x; N], [y; M])`, `(&[x; N], &[y; M])` and `&[Point<T>]` now convert into `Series`.
- **R14 Hex colors without `#`** (`bad`, `facade`) are rejected.
- **R15 Traits and placement.** `PartialEq`/`Eq`/`Copy` where possible, `#[non_exhaustive]` on `TerminalCommandError`, compact `Debug` for inline image data, and `WindowSize`/`get_window_size` moved to `termplt::terminal`.

Also added: a randomized check (300 cases) that `Series::draw_into` matches `Drawable::get_mask`.

---

## Suggested order

| Phase | Items | Goal |
|---|---|---|
| 1 | 1, 2, 3, 4, 5, 6, 8, 9, plus fmt/clippy from 42 | No hangs, no panics, correct placement |
| 2 | 7, 10, 11, 12, 14, 15, plus golden tests (43) | Readable, correct-looking plots |
| 3 | 16–24 | CLI is pleasant for everyday use |
| 4 | 25–31 | Works over SSH and tmux, fails gracefully elsewhere |
| 5 | 32–41 | Library API that's small and hard to misuse |
| 6 | 42–47 | Keep it that way |
| 7 | R1–R15 | Fix what a review of Phases 1–5 found |

## References
1. Kitty graphics protocol: querying support (`a=q` + DA1), compression (`o=z`), PNG format (`f=100`), `q` flag, Unicode placeholders. https://sw.kovidgoyal.net/kitty/graphics-protocol/
2. Liang, Y.-D., Barsky, B. A. "A New Concept and Method for Line Clipping." *ACM Transactions on Graphics* 3(1), 1984.
3. Heckbert, P. "Nice Numbers for Graph Labels." *Graphics Gems*, Academic Press, 1990.
4. clap: https://docs.rs/clap
5. lexopt: https://docs.rs/lexopt
6. Rust API Guidelines, C-COMMON-TRAITS: https://rust-lang.github.io/api-guidelines/interoperability.html
