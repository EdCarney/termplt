# termplt — Top 10 Improvement Tasks

Prioritized list of improvements for correctness, robustness, and usability.

---

## ~~1. Replace `f64::to_int_unchecked` with Safe Casts (Critical)~~ DONE

**Location:** `src/plotting/common.rs:67-78`
**PR:** [#9](https://github.com/EdCarney/termplt/pull/9)

`convert_to_u32()` and `convert_to_i32()` use `f64::to_int_unchecked`, which is **undefined behavior** for NaN, infinity, or out-of-range values. Since this is the conversion path for every pixel coordinate, a float that rounds to `-1` or `width` silently produces UB. Replace with saturating casts like `v.clamp(0.0, u32::MAX as f64) as u32` and remove the `unsafe` from the entire plotting subsystem.

---

## 2. Eliminate Panics from Public API — Use `Result` (Critical)

**Location:** `series.rs:37`, `canvas.rs:38,123`, `limits.rs:42`, `text.rs:84,170`, `images.rs:55,119`

14 `panic!` calls are reachable from public API paths: `Series::new(&[])`, `TerminalCanvas::with_graph(empty)`, `Image::new` with unsupported format combos, `Limits::new` with inverted bounds, and more. A library should never abort on bad input. Convert these to `Result::Err`.

---

## ~~3. Fix Division-by-Zero in `window_ctrl.rs` and `Point::scale_to` (High)~~ DONE

**Location:** `window_ctrl.rs:34-35`, `point.rs:112-113`
**Commits:** `7ffbdf0`, `887d752` on `ut_improvements`

`Point::scale_to` and `GraphLimits::scale_to` now use midpoint fallback when old_span is zero. `window_ctrl.rs` rejects zero-pixel terminal dimensions. Added 22 unit tests covering all zero-span and zero-pixel edge cases.

---

## 4. Remove `println!` from Library Internals (High)

**Location:** `canvas.rs:69`, `graph.rs:191`

These print to stdout from inside rendering, corrupting the Kitty escape sequence stream and producing visible artifacts. Library code must never write to stdout without opt-in.

---

## 5. Fix Base64 Encoding — Missing `=` Padding (High)

**Location:** `src/kitty_graphics/encoding.rs`

The custom base64 encoder never emits RFC 4648 padding characters. Some Kitty-compatible terminals may reject or misinterpret unpadded payloads. Consider replacing entirely with the `base64` crate (~60 lines deleted, zero bugs).

---

## 6. Fix `unwrap()` Inside `Result`-Returning Functions (High)

**Location:** `canvas.rs:143,160`, `graph.rs:236`

`draw()` returns `Result` but internally calls `.unwrap()` on fallible operations like `graph.limits()` and `series.get_mask()`. This defeats error propagation — errors panic instead of being returned to the caller. Use `?` and `collect::<Result<Vec<_>>>()`.

---

## ~~7. Fix Line Drawing for Right-to-Left and Vertical Lines (High)~~ DONE

**Location:** `src/plotting/line.rs:139-175`
**Commit:** `b7dba48` on `ut_improvements`

Replaced slope/intercept line drawing with Bresenham's algorithm, fixing vertical line division-by-zero and right-to-left empty-range bugs. Added 10 unit tests covering all orientations.

---

## 8. Complete Unimplemented Public Enum Variants (Medium)

**Location:** `canvas.rs:38`, `text.rs:253,276`

`BufferType::TopBottom`, `LeftRight`, `TopBottomLeftRight` and `TextPositioning::LeftAligned` are public variants that `panic!("Not implemented")` at runtime. Either implement them or remove from the public API.

---

## 9. Add a Crate-Level Error Type (Medium)

**Location:** `common.rs:3`

`Box<dyn Error>` erases type information, prevents pattern matching, forces heap allocation, and breaks `Send + Sync` for async. Define a `TermpltError` enum (consider `thiserror`) unifying `WindowCtrlError`, `TerminalCommandError`, and `ImageError`.

---

## 10. Add Integration Tests and Fix `LineStyle::Dashed` / `grid_lines` Shift (Medium)

**Location:** No `tests/` directory

All tests are unit tests; no rendering integration tests exist. `LineStyle::Dashed` is publicly exposed but contains `todo!()`. `Graph::shift_by` shifts `data` and `graph_limits` but **not** `grid_lines`, causing misaligned grids after translation.

---

## Recommended Implementation Order (remaining)

1. **Task 4** — 2 lines deleted, immediate rendering fix
2. **Task 5** — Protocol correctness for Kitty transmissions
3. **Task 6** — Error propagation correctness
4. **Task 8** — Remove panic traps from public API
5. **Task 2** — Broad API hardening
6. **Task 9** — API design cleanup
7. **Task 10** — Tests and documentation
