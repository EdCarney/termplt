# TrueType Text, Titles and Axis Names Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace termplt's bitmap digit font with an embedded, anti-aliased TrueType font (a trimmed Go Regular), then use it for plot titles and axis names in the library and the CLI.

**Architecture:**
- `plotting/font.rs` parses fonts with `ab_glyph`, falls back to the built-in Go font for missing characters, and rasterizes one line of text into an 8-bit coverage bitmap.
- The canvas blends coverage into its RGB pixels in linear light, using integer-only sRGB tables (`plotting/srgb.rs`).
- `canvas.rs` places all text as `PlacedText` in layout bands whose gaps scale with the font size. Titles and axis names wrap with a pure `wrap()` function.
- Series, axes and grid still draw solid pixels exactly as before.

**Tech Stack:** Rust 2024 (MSRV 1.88), `ab_glyph` 0.2 (`std` feature only), `image` (PNG), `clap`. Python 3 with a pinned `fonttools`, used only by the font-subsetting script.

**Spec:** `docs/superpowers/specs/2026-09-25-text-rendering-design.md`. Read it before starting a task. It explains why each decision was made; this plan says how.

## Global Constraints

- **Rust:** `edition = "2024"`, `rust-version = "1.88"`. Don't use std APIs newer than 1.88; clippy's `incompatible_msrv` and the CI MSRV job enforce this.
- **The only new dependency:** `ab_glyph = { version = "0.2.32", default-features = false, features = ["std"] }`.
- **Package license:** `license = "MIT AND BSD-3-Clause"`.
- **Font source:**
  - Go Regular 2.010 from `golang/image` at commit `41969df76e82aeec85fa3821b1e24955ea993001`, SHA-256 `197d9f3703b4c00af609178876a8d73e396f64fe438b2c871778566632374be3`.
  - Subset with `fonttools==4.66.0`: `pyftsubset --no-hinting --name-IDs='*' --notdef-outline --unicodes-file=assets/fonts/charset.txt`.
- **Sizes:**
  - `DEFAULT_FONT_SIZE = 14` and `MAX_FONT_SIZE = 400`, both in `plotting::text`.
  - Title size = `round(1.2 × tick size)`.
  - Terminal text size = `round(pix_per_row / 1.2)`, clamped to `8..=400`.
- **Gaps** (em = the tick-label size):
  - tick labels to plot: 0.35 em;
  - axis name to tick labels: 0.4 em;
  - title to plot: 0.6 em;
  - offset label: 0.3 em.
- **Line layout:**
  - Shared lines move apart when their boxes are less than 0.5 em apart.
  - Wrapped lines are 1.2 em apart, baseline to baseline.
  - A title gets at most 3 lines and an axis name at most 2.
- **Text content:**
  - Every generated number uses the Unicode minus `−` (U+2212).
  - The ellipsis is `…` (U+2026).
- **Unchanged:**
  - Golden images compare exactly: `TOLERANCE` in `tests/golden.rs` stays at 0.001.
  - `Series::draw_into` and its equivalence test with `get_mask`.
  - `BufferType`.
  - No build scripts.
- **Gate:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo test --no-default-features` and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` must pass before every commit.
  - Exception: Tasks 1–3 and 7 add crate-private code whose first caller comes in a later task, so run clippy with `-A dead_code` in those tasks only.
- **Commits** end with the line `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. PR bodies end with `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- **Behavior:** follow matplotlib's defaults unless the spec lists a deliberate difference.
- **Rendering changes:** after any intentional rendering change, regenerate snapshots with `TERMPLT_UPDATE_SNAPSHOTS=1 cargo test --test golden` and README images with `scripts/readme_images.sh`, then **open and look at every changed PNG** before committing.

## Review Focus

These are inputs that no task's main tests exercise but that are likely to bite users. Each one is pinned by a test in the task noted.

1. **A title or name that starts with `-`**, such as `--title "-5 dB"`. It should be accepted as text, not read as a flag (Task 12).
2. **Characters no font has**, such as CJK or emoji in a title. They should draw a visible box (the `.notdef` glyph), not nothing (Task 2).
3. **Tabs and other control characters in text.** They should take the space of a space character (Task 1).
4. **Quoted or padded CSV headers**, such as `"time (s)" , temp`. The axis name should be the clean text `time (s)` (Task 11).
5. **A terminal that reports 0 px or huge rows.** The text size should stay within 8–400 px instead of vanishing or overflowing (Task 9).

## File Map

| File | Responsibility | Tasks |
|---|---|---|
| `assets/fonts/charset.txt` | Characters kept in the embedded font | 1 |
| `assets/fonts/go-regular-subset.ttf` | The embedded font (generated) | 1 |
| `assets/fonts/LICENSE-Go` | The font's BSD-3-Clause license, verbatim | 1 |
| `scripts/subset_font.sh` | Rebuilds the two font files from the pinned source | 1 |
| `tests/fixtures/digits-only.ttf` | Go with only `0-9`, for fallback tests (generated) | 1 |
| `src/plotting/font.rs` | `Font`, glyph fallback, metrics, measuring, rasterizing to `Coverage`, rotation | 1, 2, 8 |
| `src/plotting/srgb.rs` | Linear-light blending with integer tables | 3 |
| `src/plotting/text.rs` | `TextStyle`, `TextPositioning`, `Label`, size constants, `wrap()` | 4, 7 |
| `src/plotting/numbers.rs` | **Deleted** (the bitmap font) | 4 |
| `src/plotting/canvas.rs` | `Canvas::blend`, `PlacedText`, `TerminalCanvas` font and size, layout | 3, 4, 8 |
| `src/plotting/ticks.rs` | Unicode minus | 5 |
| `src/plotting/graph.rs` | `with_title`, `with_x_label`, `with_y_label` | 8 |
| `src/plot.rs` | `Plot::title`, `x_label`, `y_label`, `font`, `font_size`; terminal-matched size | 9 |
| `src/terminal.rs` | `Terminal::text_size` | 9 |
| `src/error.rs` | `Error::InvalidFont` | 1 |
| `src/bin/termplt/{cli,main,data}.rs` | Text flags, automatic names from headers | 11, 12 |
| `tests/{golden,properties,cli}.rs` | Snapshots, properties, CLI end-to-end | 4, 10, 12 |
| `.github/workflows/release.yml` | License files in the archives | 6 |
| `README.md`, `CLAUDE.md`, `IMPROVEMENTS.md`, `CHANGELOG.md` | Docs | 4, 6, 13 |

## Branches and PRs

- **PR 1 (Tasks 1–6):**
  - Branch `text-engine` from `text-rendering-spec`, so the spec and this plan land with the first PR.
  - Base: `main`.
- **PR 2 (Tasks 7–10):** branch `titles-and-axis-names`.
  - If PR 1 is merged: from an updated `main`.
  - If not: from `text-engine`, with the PR's base set to `text-engine`.
- **PR 3 (Tasks 11–13):** branch `cli-text` from `main` once PR 2 is merged, or stacked the same way.
- **No release at the end of this plan.** 0.3.0 is tagged after spec 2 (the legend).

```bash
git checkout text-rendering-spec && git pull --ff-only
git checkout -b text-engine
```

---

# PR 1: Text engine swap

### Task 1: Embedded Go font and the `Font` type

**Files:**
- Create: `assets/fonts/charset.txt`, `assets/fonts/LICENSE-Go`, `scripts/subset_font.sh`
- Generate: `assets/fonts/go-regular-subset.ttf`, `tests/fixtures/digits-only.ttf`
- Create: `src/plotting/font.rs`
- Modify: `src/plotting.rs`, `src/prelude.rs`, `src/error.rs`, `Cargo.toml`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub struct Font` (Clone, Default, Debug, PartialEq, Send + Sync) and `Font::from_bytes(data: impl Into<Vec<u8>>) -> Result<Font>`.
  - `pub(crate) fn Font::metrics(&self, size: u32) -> LineMetrics` and `pub(crate) fn Font::width(&self, text: &str, size: u32) -> f32`.
  - `pub(crate) struct LineMetrics { pub ascent: f32, pub descent: f32, pub digit_height: f32 }` with `fn height(&self) -> u32`.
  - Private helpers for Task 2: `fn scale(&FontArc, u32) -> PxScale`, `fn positions(&self, &str, u32) -> Vec<PositionedGlyph<'_>>`, `struct PositionedGlyph<'a> { font: &'a FontArc, id: GlyphId, x: f32, advance: f32 }`.
  - `Error::InvalidFont(String)`.

- [ ] **Step 1: Add the character list, license and subsetting script**

`assets/fonts/charset.txt`:

```text
# Characters kept in the embedded font (assets/fonts/go-regular-subset.ttf).
# scripts/subset_font.sh passes this file to pyftsubset --unicodes-file, and a unit test in
# src/plotting/font.rs checks that the embedded font has a glyph for every entry.

# Basic Latin and Latin-1 Supplement (accents, ° ± µ ² ³ × ÷ ...)
U+0020-007E
U+00A0-00FF
# Greek capitals (U+03A2 is unassigned) and small letters
U+0391-03A1
U+03A3-03A9
U+03B1-03C9
# Punctuation: – — ‘ ’ ‚ ‛ “ ” • … ‰
U+2013-2014
U+2018-201D
U+2022
U+2026
U+2030
# Superscripts ⁰ ⁴-⁹ and subscripts ₀-₉ (¹ ² ³ are in Latin-1)
U+2070
U+2074-2079
U+2080-2089
# Euro sign
U+20AC
# Arrows ← ↑ → ↓
U+2190-2193
# Math: ∆ ∑ − √ ∞ ≈ ≠ ≤ ≥
U+2206
U+2211
U+2212
U+221A
U+221E
U+2248
U+2260
U+2264-2265
```

`assets/fonts/LICENSE-Go` (verbatim from the font's `name` record 13 and `golang.org/x/image/font/gofont/ttfs/README`):

```text
Copyright (c) 2016 Bigelow & Holmes Inc.. All rights reserved.

Distribution of this font is governed by the following license. If you do not
agree to this license, including the disclaimer, do not distribute or modify
this font.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

	* Redistributions of source code must retain the above copyright notice,
	  this list of conditions and the following disclaimer.

	* Redistributions in binary form must reproduce the above copyright notice,
	  this list of conditions and the following disclaimer in the documentation
	  and/or other materials provided with the distribution.

	* Neither the name of Google Inc. nor the names of its contributors may be
	  used to endorse or promote products derived from this software without
	  specific prior written permission.

DISCLAIMER: THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO,
THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

`scripts/subset_font.sh` (make it executable with `chmod +x`):

```bash
#!/usr/bin/env bash
# Rebuilds the embedded font (assets/fonts/go-regular-subset.ttf) and the digits-only test font
# (tests/fixtures/digits-only.ttf) from the pinned Go Regular release. Needs python3 and network
# access; a pinned fonttools is installed into a temporary virtualenv. The output is
# deterministic, so an unchanged run leaves `git status` clean.
set -euo pipefail
cd "$(dirname "$0")/.."

# golang/image commit "font/gofont: upgrade to version 2.010"
commit=41969df76e82aeec85fa3821b1e24955ea993001
sha256=197d9f3703b4c00af609178876a8d73e396f64fe438b2c871778566632374be3
url="https://raw.githubusercontent.com/golang/image/$commit/font/gofont/ttfs/Go-Regular.ttf"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
curl -sSfL -o "$work/Go-Regular.ttf" "$url"
if command -v sha256sum >/dev/null; then
    echo "$sha256  $work/Go-Regular.ttf" | sha256sum -c - >/dev/null
else
    echo "$sha256  $work/Go-Regular.ttf" | shasum -a 256 -c - >/dev/null
fi

python3 -m venv "$work/venv"
"$work/venv/bin/pip" install --quiet fonttools==4.66.0

# --name-IDs='*' keeps the copyright and license records; --notdef-outline keeps the box drawn
# for characters the font lacks; ab_glyph ignores hinting, so it is dropped
subset() {
    "$work/venv/bin/pyftsubset" "$work/Go-Regular.ttf" --no-hinting --name-IDs='*' \
        --notdef-outline "$@"
}
subset --unicodes-file=assets/fonts/charset.txt --output-file=assets/fonts/go-regular-subset.ttf
mkdir -p tests/fixtures
subset --unicodes=U+0030-0039 --output-file=tests/fixtures/digits-only.ttf
ls -l assets/fonts/go-regular-subset.ttf tests/fixtures/digits-only.ttf
```

- [ ] **Step 2: Generate the fonts**

Run: `chmod +x scripts/subset_font.sh && scripts/subset_font.sh`

Expected: two files are listed, `go-regular-subset.ttf` at about 29 KB and `digits-only.ttf` at a few KB. Run it a second time: `git status --short assets tests/fixtures` shows the same two new files, and `cmp` against copies from the first run shows identical bytes.

- [ ] **Step 3: Add the dependency, the license field and the error variant**

In `Cargo.toml`, change `license = "MIT"` to `license = "MIT AND BSD-3-Clause"` and add under `[dependencies]` (keep the list alphabetical):

```toml
ab_glyph = { version = "0.2.32", default-features = false, features = ["std"] }
```

In `src/error.rs`, add after the `InvalidPath(String)` variant:

```rust
    /// Font data that is not a TrueType or OpenType font.
    InvalidFont(String),
```

and in `Display` after the `Error::InvalidPath` arm:

```rust
            Error::InvalidFont(msg) => write!(f, "invalid font: {msg}"),
```

- [ ] **Step 4: Write the failing tests**

Create `src/plotting/font.rs` with only the tests for now (the module won't compile until Step 6, which counts as the failing run):

```rust
//! TrueType fonts: the built-in Go font, fonts supplied by the user, and laying out a line of
//! text.

#[cfg(test)]
mod tests {
    use super::*;

    const DIGITS_ONLY: &[u8] = include_bytes!("../../tests/fixtures/digits-only.ttf");

    /// The code points listed in `assets/fonts/charset.txt`.
    fn charset() -> Vec<char> {
        let text = include_str!("../../assets/fonts/charset.txt");
        let hex = |s: &str| u32::from_str_radix(s.trim().trim_start_matches("U+"), 16).unwrap();
        let mut chars = Vec::new();
        for line in text.lines() {
            let entry = line.split('#').next().unwrap_or("").trim();
            if entry.is_empty() {
                continue;
            }
            let (first, last) = match entry.split_once('-') {
                Some((a, b)) => (hex(a), hex(b)),
                None => (hex(entry), hex(entry)),
            };
            chars.extend((first..=last).filter_map(char::from_u32));
        }
        chars
    }

    #[test]
    fn built_in_font_covers_the_charset() {
        let chars = charset();
        // the list parsed: Latin-1 alone is 191 characters
        assert!(chars.len() > 250, "{} characters", chars.len());
        let missing: String = chars.iter().filter(|&&c| go().glyph_id(c).0 == 0).collect();
        assert!(missing.is_empty(), "missing from the built-in font: {missing}");
    }

    #[test]
    fn from_bytes_rejects_other_data() {
        let err = Font::from_bytes(b"not a font".to_vec()).unwrap_err();
        assert!(matches!(err, Error::InvalidFont(_)), "{err}");
    }

    #[test]
    fn characters_a_font_lacks_are_drawn_with_go() {
        let digits = Font::from_bytes(DIGITS_ONLY).unwrap();
        let go = Font::default();
        assert!(go.width("A", 14) > 0.0);
        assert_eq!(digits.width("A", 14), go.width("A", 14));
        // no font here has U+4E00; it takes the width of the .notdef box
        assert!(digits.width("\u{4e00}", 14) > 0.0);
    }

    #[test]
    fn widths_add_up_without_kerning() {
        // Go has no kerning, so a line is exactly as wide as its glyphs' advances
        let font = Font::default();
        let zero = font.width("0", 14);
        assert!(zero > 0.0);
        assert_eq!(font.width("00", 14), 2.0 * zero);
        assert_eq!(font.width("", 14), 0.0);
        assert!((font.width("0", 28) - 2.0 * zero).abs() < 1e-3);
    }

    #[test]
    fn control_characters_take_the_space_of_a_space() {
        let font = Font::default();
        assert_eq!(font.width("a\tb", 14), font.width("a b", 14));
    }

    #[test]
    fn metrics_scale_with_the_size() {
        let (small, large) = (Font::default().metrics(14), Font::default().metrics(28));
        assert!(small.ascent > 0.0 && small.descent < 0.0);
        assert!((large.ascent - 2.0 * small.ascent).abs() < 1e-3);
        assert_eq!(small.height(), (small.ascent - small.descent).ceil() as u32);
        // Go's 0 is about 0.76 em tall
        let em = small.digit_height / 14.0;
        assert!(em > 0.7 && em < 0.8, "{em}");
    }

    #[test]
    fn clones_are_equal_and_separate_loads_are_not() {
        let a = Font::from_bytes(DIGITS_ONLY).unwrap();
        assert_eq!(a, a.clone());
        assert_ne!(a, Font::from_bytes(DIGITS_ONLY).unwrap());
        assert_eq!(Font::default(), Font::default());
        assert_ne!(a, Font::default());
    }

    #[test]
    fn font_is_send_and_sync() {
        fn check<T: Send + Sync + 'static>() {}
        check::<Font>();
    }
}
```

Register the module in `src/plotting.rs` (keep alphabetical order):

```rust
/// Fonts for text on plots.
pub mod font;
```

and add `font::Font` to the `plotting::{...}` list in `src/prelude.rs`.

- [ ] **Step 5: Run the tests to confirm they fail**

Run: `cargo test --lib plotting::font`

Expected: the build fails with errors like "cannot find function `go`" and "cannot find type `Font`". This is the failing state; the implementation doesn't exist yet.

- [ ] **Step 6: Implement `Font`**

Put this above the tests module in `src/plotting/font.rs`:

```rust
use crate::{Error, Result};
use ab_glyph::{Font as _, FontArc, GlyphId, PxScale, ScaleFont as _};
use std::{
    fmt,
    sync::{Arc, OnceLock},
};

/// Go Regular 2.010, trimmed to the characters in `assets/fonts/charset.txt` by
/// `scripts/subset_font.sh`. BSD-3-Clause, © 2016 Bigelow & Holmes Inc.
const GO_REGULAR: &[u8] = include_bytes!("../../assets/fonts/go-regular-subset.ttf");

/// The built-in font, parsed once.
fn go() -> &'static FontArc {
    static GO: OnceLock<FontArc> = OnceLock::new();
    GO.get_or_init(|| FontArc::try_from_slice(GO_REGULAR).expect("the built-in font is valid"))
}

/// The font text is drawn with. The default is the built-in Go font, which covers Latin-1,
/// Greek and common math symbols. [`Font::from_bytes`] loads another font; characters it
/// lacks are drawn with Go.
#[derive(Clone, Default)]
pub struct Font {
    /// `None` for the built-in font.
    user: Option<Arc<FontArc>>,
}

impl Font {
    /// Loads a TrueType or OpenType font (for a collection, its first face).
    pub fn from_bytes(data: impl Into<Vec<u8>>) -> Result<Font> {
        let font = FontArc::try_from_vec(data.into())
            .map_err(|_| Error::InvalidFont("not a TrueType or OpenType font".to_string()))?;
        Ok(Font {
            user: Some(Arc::new(font)),
        })
    }

    fn primary(&self) -> &FontArc {
        match &self.user {
            Some(font) => font,
            None => go(),
        }
    }

    /// The font and glyph `c` is drawn with: this font's, else Go's, else this font's `.notdef`
    /// glyph (usually a box).
    fn glyph(&self, c: char) -> (&FontArc, GlyphId) {
        let primary = self.primary();
        let id = primary.glyph_id(c);
        if id.0 != 0 {
            return (primary, id);
        }
        let id = go().glyph_id(c);
        if id.0 != 0 {
            (go(), id)
        } else {
            (primary, GlyphId(0))
        }
    }

    /// Vertical metrics of a line at `size` pixels (the em size, like CSS `font-size`).
    pub(crate) fn metrics(&self, size: u32) -> LineMetrics {
        let font = self.primary();
        let scaled = font.as_scaled(scale(font, size));
        let (zero_font, zero) = self.glyph('0');
        // outline bounds are in font units with y pointing up, so the height is negative
        let digit_height = zero_font
            .outline(zero)
            .map_or(0.0, |outline| outline.bounds.height().abs())
            * zero_font.as_scaled(scale(zero_font, size)).v_scale_factor();
        LineMetrics {
            ascent: scaled.ascent(),
            descent: scaled.descent(),
            digit_height,
        }
    }

    /// Width in pixels of `text` on one line at `size` pixels.
    pub(crate) fn width(&self, text: &str, size: u32) -> f32 {
        self.positions(text, size)
            .last()
            .map_or(0.0, |glyph| glyph.x + glyph.advance)
    }

    /// Each glyph of `text` with its font and pen position.
    fn positions(&self, text: &str, size: u32) -> Vec<PositionedGlyph<'_>> {
        let mut glyphs: Vec<PositionedGlyph<'_>> = Vec::with_capacity(text.len());
        let mut x = 0.0;
        for c in text.chars() {
            // callers split lines at `\n`; any other control character takes a space
            let c = if c.is_control() { ' ' } else { c };
            let (font, id) = self.glyph(c);
            let scaled = font.as_scaled(scale(font, size));
            if let Some(prev) = glyphs.last()
                && std::ptr::eq(prev.font, font)
            {
                x += scaled.kern(prev.id, id);
            }
            let advance = scaled.h_advance(id);
            glyphs.push(PositionedGlyph { font, id, x, advance });
            x += advance;
        }
        glyphs
    }
}

impl PartialEq for Font {
    /// Fonts are equal when both are the built-in font, or both are the same loaded font (a
    /// clone). Loading the same bytes twice gives two different fonts.
    fn eq(&self, other: &Font) -> bool {
        match (&self.user, &other.user) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.user {
            None => f.write_str("Font(Go, built in)"),
            Some(_) => f.write_str("Font(loaded)"),
        }
    }
}

/// Vertical metrics of a line of text, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LineMetrics {
    /// From the baseline up to the top of the line box.
    pub ascent: f32,
    /// From the baseline down to the bottom of the line box (negative).
    pub descent: f32,
    /// Height of the digit `0`, for centering numbers.
    pub digit_height: f32,
}

impl LineMetrics {
    /// Height of the line box in whole pixels.
    pub fn height(&self) -> u32 {
        (self.ascent - self.descent).ceil() as u32
    }
}

/// A glyph with the font it comes from and its pen position on the line.
struct PositionedGlyph<'a> {
    font: &'a FontArc,
    id: GlyphId,
    x: f32,
    advance: f32,
}

/// The `ab_glyph` scale for an em size of `size` pixels. `PxScale` measures from descent to
/// ascent, which differs from the em size by a per-font factor.
fn scale(font: &FontArc, size: u32) -> PxScale {
    let units_per_em = font.units_per_em().unwrap_or(1000.0);
    PxScale::from(size as f32 * font.height_unscaled() / units_per_em)
}
```

- [ ] **Step 7: Run the tests to confirm they pass**

Run: `cargo test --lib plotting::font`

Expected: all 8 tests pass.

- [ ] **Step 8: Run the gate and commit**

Run the gate from Global Constraints, with `cargo clippy --all-targets -- -D warnings -A dead_code` for this task.

```bash
git add assets/fonts scripts/subset_font.sh tests/fixtures src/plotting/font.rs src/plotting.rs src/prelude.rs src/error.rs Cargo.toml Cargo.lock
git commit -F - <<'EOF'
Embed a trimmed Go font and add the Font type

Go Regular 2.010 (BSD-3-Clause), trimmed to Latin-1, Greek and common math symbols by
scripts/subset_font.sh from a pinned, hash-checked source. Font::from_bytes loads other
fonts, and characters they lack fall back to Go.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 2: Rasterize a line of text

**Files:**
- Modify: `src/plotting/font.rs`

**Interfaces:**
- Consumes: `Font::positions`, `Font::metrics` and `scale` from Task 1.
- Produces:
  - `pub(crate) struct Coverage { pub width: u32, pub height: u32, pub data: Vec<u8> }`, row-major from the top row, 0 to 255, with `pub fn get(&self, x: u32, y: u32) -> u8`.
  - `pub(crate) fn Font::rasterize(&self, text: &str, size: u32) -> Coverage`. The bitmap is `ceil(width)` wide and `metrics.height()` tall, with the baseline at row `ascent`.

- [ ] **Step 1: Write the failing tests** (add to the tests module in `font.rs`)

```rust
    #[test]
    fn a_line_rasterizes_into_its_box() {
        let font = Font::default();
        let metrics = font.metrics(14);
        let line = font.rasterize("0", 14);
        assert_eq!(line.width, font.width("0", 14).ceil() as u32);
        assert_eq!(line.height, metrics.height());
        // the 0 is inked between the baseline and the digit height above it
        let inked: Vec<u32> = (0..line.height)
            .filter(|&y| (0..line.width).any(|x| line.get(x, y) > 0))
            .collect();
        assert!(!inked.is_empty());
        for &y in &inked {
            let y = y as f32;
            assert!(y < metrics.ascent + 1.0, "row {y} is below the baseline");
            assert!(y > metrics.ascent - metrics.digit_height - 1.0, "row {y} is too high");
        }
        // anti-aliased: edge pixels are partly covered
        assert!(line.data.iter().any(|&c| c > 0 && c < 255));
    }

    #[test]
    fn an_empty_line_has_no_pixels() {
        let line = Font::default().rasterize("", 14);
        assert_eq!(line.width, 0);
        assert!(line.data.is_empty());
    }

    #[test]
    fn characters_no_font_has_are_drawn_as_a_box() {
        // CJK is not in the built-in font: the .notdef box must still show
        let line = Font::default().rasterize("\u{4e00}", 14);
        assert!(line.data.iter().any(|&c| c > 0));
    }
```

- [ ] **Step 2: Run them to confirm they fail**

Run: `cargo test --lib plotting::font`

Expected: the build fails with "no method named `rasterize`" and "cannot find type `Coverage`".

- [ ] **Step 3: Implement `Coverage` and `rasterize`**

Add `point` to the `ab_glyph` import (`use ab_glyph::{Font as _, FontArc, GlyphId, PxScale, ScaleFont as _, point};`), then add the method inside `impl Font`:

```rust
    /// Draws `text` on one line at `size` pixels. The bitmap is as wide as the text and as
    /// tall as the line box, with the baseline `ascent` rows from the top; ink outside that box
    /// (rare overhangs) is dropped.
    pub(crate) fn rasterize(&self, text: &str, size: u32) -> Coverage {
        let metrics = self.metrics(size);
        let glyphs = self.positions(text, size);
        let width = glyphs
            .last()
            .map_or(0.0, |glyph| glyph.x + glyph.advance)
            .ceil() as u32;
        let height = metrics.height();
        let mut data = vec![0u8; width as usize * height as usize];
        for glyph in &glyphs {
            let placed = glyph
                .id
                .with_scale_and_position(scale(glyph.font, size), point(glyph.x, metrics.ascent));
            let Some(outline) = glyph.font.outline_glyph(placed) else {
                continue;
            };
            let bounds = outline.px_bounds();
            outline.draw(|gx, gy, coverage| {
                let x = bounds.min.x as i64 + i64::from(gx);
                let y = bounds.min.y as i64 + i64::from(gy);
                if (0..i64::from(width)).contains(&x) && (0..i64::from(height)).contains(&y) {
                    let i = y as usize * width as usize + x as usize;
                    let add = (coverage.clamp(0.0, 1.0) * 255.0).round() as u8;
                    data[i] = data[i].saturating_add(add);
                }
            });
        }
        Coverage {
            width,
            height,
            data,
        }
    }
```

and after `LineMetrics`:

```rust
/// A line of text drawn as 8-bit coverage (0 = empty, 255 = fully covered), row-major from the
/// top row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Coverage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Coverage {
    /// Coverage at column `x`, row `y` (from the top).
    pub fn get(&self, x: u32, y: u32) -> u8 {
        self.data[y as usize * self.width as usize + x as usize]
    }
}
```

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `cargo test --lib plotting::font`

Expected: all 11 tests pass.

- [ ] **Step 5: Run the gate** (clippy with `-A dead_code`) **and commit**

```bash
git add src/plotting/font.rs
git commit -F - <<'EOF'
Rasterize a line of text into a coverage bitmap

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 3: Blend into the canvas in linear light

**Files:**
- Create: `src/plotting/srgb.rs`
- Modify: `src/plotting.rs` (add the private module `mod srgb;`), `src/plotting/canvas.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub(crate) fn srgb::blend(bg: u8, fg: u8, coverage: u8) -> u8` for one color channel.
  - `pub fn Canvas::blend(&mut self, x: u32, y: u32, color: RGB8, coverage: u8)`, with (0, 0) at the lower left. Points outside the canvas are ignored.

- [ ] **Step 1: Write the failing tests**

Create `src/plotting/srgb.rs` with the tests:

```rust
//! Mixing colors in linear light, using integer math only so that every platform produces the
//! same bytes.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_coverage_keeps_the_background_and_full_coverage_paints() {
        for v in [0, 1, 17, 128, 254, 255] {
            assert_eq!(blend(v, 200, 0), v);
            assert_eq!(blend(200, v, 255), v);
        }
    }

    #[test]
    fn partial_coverage_mixes_in_linear_light() {
        // half-covered white on black is 50% light, which is sRGB 188; plain averaging gives
        // 128, only 22% light, so anti-aliased text would look thin
        assert_eq!(blend(0, 255, 128), 188);
        assert_eq!(blend(0, 255, 64), 137);
        assert_eq!(blend(0, 255, 191), 224);
        assert_eq!(blend(255, 0, 128), 187);
    }

    #[test]
    fn table_matches_the_srgb_formula() {
        for (i, &value) in SRGB_TO_LINEAR.iter().enumerate() {
            let c = i as f64 / 255.0;
            let linear = if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            };
            assert!((f64::from(value) - linear * 65535.0).abs() <= 1.0, "entry {i}");
        }
        assert!(SRGB_TO_LINEAR.windows(2).all(|w| w[0] < w[1]));
    }
}
```

Add `mod srgb;` to `src/plotting.rs`, after `mod line_positioning;`. In the tests module of `src/plotting/canvas.rs` add:

```rust
    #[test]
    fn blend_mixes_into_one_pixel() {
        let mut canvas = Canvas::new(2, 2, colors::BLACK);
        canvas.blend(0, 0, colors::WHITE, 128);
        canvas.blend(5, 5, colors::WHITE, 255); // outside: ignored
        // (0, 0) is the lower-left pixel, stored in the last row
        assert_eq!(canvas.get_bytes(), [0, 0, 0, 0, 0, 0, 188, 188, 188, 0, 0, 0]);
    }
```

- [ ] **Step 2: Run them to confirm they fail**

Run: `cargo test --lib -- srgb canvas::tests::blend`

Expected: the build fails because `blend`, `SRGB_TO_LINEAR` and `Canvas::blend` don't exist.

- [ ] **Step 3: Implement**

Above the tests in `src/plotting/srgb.rs`:

```rust
/// sRGB value to linear light, scaled to `0..=65535`: `round(65535 * linear(i / 255))`, where
/// `linear(c)` is `c / 12.92` up to 0.04045 and `((c + 0.055) / 1.055)^2.4` above. Committed
/// rather than computed so no platform-dependent `powf` rounding reaches the pixels.
const SRGB_TO_LINEAR: [u16; 256] = [
    0, 20, 40, 60, 80, 99, 119, 139, 159, 179, 199, 219,
    241, 264, 288, 313, 340, 367, 396, 427, 458, 491, 526, 562,
    599, 637, 677, 718, 761, 805, 851, 898, 947, 997, 1048, 1101,
    1156, 1212, 1270, 1330, 1391, 1453, 1517, 1583, 1651, 1720, 1790, 1863,
    1937, 2013, 2090, 2170, 2250, 2333, 2418, 2504, 2592, 2681, 2773, 2866,
    2961, 3058, 3157, 3258, 3360, 3464, 3570, 3678, 3788, 3900, 4014, 4129,
    4247, 4366, 4488, 4611, 4736, 4864, 4993, 5124, 5257, 5392, 5530, 5669,
    5810, 5953, 6099, 6246, 6395, 6547, 6700, 6856, 7014, 7174, 7335, 7500,
    7666, 7834, 8004, 8177, 8352, 8528, 8708, 8889, 9072, 9258, 9445, 9635,
    9828, 10022, 10219, 10417, 10619, 10822, 11028, 11235, 11446, 11658, 11873, 12090,
    12309, 12530, 12754, 12980, 13209, 13440, 13673, 13909, 14146, 14387, 14629, 14874,
    15122, 15371, 15623, 15878, 16135, 16394, 16656, 16920, 17187, 17456, 17727, 18001,
    18277, 18556, 18837, 19121, 19407, 19696, 19987, 20281, 20577, 20876, 21177, 21481,
    21787, 22096, 22407, 22721, 23038, 23357, 23678, 24002, 24329, 24658, 24990, 25325,
    25662, 26001, 26344, 26688, 27036, 27386, 27739, 28094, 28452, 28813, 29176, 29542,
    29911, 30282, 30656, 31033, 31412, 31794, 32179, 32567, 32957, 33350, 33745, 34143,
    34544, 34948, 35355, 35764, 36176, 36591, 37008, 37429, 37852, 38278, 38706, 39138,
    39572, 40009, 40449, 40891, 41337, 41785, 42236, 42690, 43147, 43606, 44069, 44534,
    45002, 45473, 45947, 46423, 46903, 47385, 47871, 48359, 48850, 49344, 49841, 50341,
    50844, 51349, 51858, 52369, 52884, 53401, 53921, 54445, 54971, 55500, 56032, 56567,
    57105, 57646, 58190, 58737, 59287, 59840, 60396, 60955, 61517, 62082, 62650, 63221,
    63795, 64372, 64952, 65535,
];

/// The sRGB value whose linear light is nearest `linear`.
fn to_srgb(linear: u32) -> u8 {
    let i = SRGB_TO_LINEAR.partition_point(|&v| u32::from(v) < linear);
    match i {
        0 => 0,
        256 => 255,
        _ => {
            let below = u32::from(SRGB_TO_LINEAR[i - 1]);
            let above = u32::from(SRGB_TO_LINEAR[i]);
            if above - linear < linear - below {
                i as u8
            } else {
                (i - 1) as u8
            }
        }
    }
}

/// One color channel of `fg` drawn over `bg` with `coverage` (0 = none, 255 = all), mixed in
/// linear light.
pub(crate) fn blend(bg: u8, fg: u8, coverage: u8) -> u8 {
    let bg = u32::from(SRGB_TO_LINEAR[usize::from(bg)]);
    let fg = u32::from(SRGB_TO_LINEAR[usize::from(fg)]);
    let a = u32::from(coverage);
    to_srgb((bg * (255 - a) + fg * a + 127) / 255)
}
```

`rustfmt` would reflow the table to one number per line. Put `#[rustfmt::skip]` on the `const SRGB_TO_LINEAR` line to keep 12 per row.

In `src/plotting/canvas.rs`, add `srgb` to the `use super::{...}` list, and add this method to `impl Canvas` after `put`:

```rust
    /// Draws `color` over the pixel at (`x`, `y`) with `coverage` (0 = none, 255 = all), mixing
    /// in linear light. (0, 0) is the lower-left corner; points outside the canvas are ignored.
    pub fn blend(&mut self, x: u32, y: u32, color: RGB8, coverage: u8) {
        if x < self.width && y < self.height {
            // rows are stored from the top
            let row = (self.height - 1 - y) as usize;
            let i = (row * self.width as usize + x as usize) * 3;
            for (channel, fg) in self.bytes[i..i + 3].iter_mut().zip([color.r, color.g, color.b]) {
                *channel = srgb::blend(*channel, fg, coverage);
            }
        }
    }
```

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `cargo test --lib -- srgb canvas::tests::blend`

Expected: 4 tests pass.

- [ ] **Step 5: Run the gate** (clippy with `-A dead_code`) **and commit**

```bash
git add src/plotting/srgb.rs src/plotting.rs src/plotting/canvas.rs
git commit -F - <<'EOF'
Blend coverage into the canvas in linear light

Integer math and a committed sRGB table, so every platform produces the same bytes.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 4: Draw all text with the font

This replaces the bitmap font. The public `TextStyle` and `Label` change, as in spec § Public API. The canvas gains a font and a base size, and every label is rasterized and blended.

**Files:**
- Rewrite: `src/plotting/text.rs`
- Delete: `src/plotting/numbers.rs`
- Modify: `src/plotting.rs`, `src/plotting/canvas.rs`, `tests/properties.rs`, `README.md` (the features bullet)
- Regenerate: `tests/snapshots/*.png`, `docs/images/*.png`

**Interfaces:**
- Consumes: `Font`, `Font::{metrics, width, rasterize}`, `Coverage`, `Canvas::blend`.
- Produces (public):
  - `DEFAULT_FONT_SIZE: u32 = 14` and `MAX_FONT_SIZE: u32 = 400`.
  - `TextStyle::new(color: RGB8, size: u32)`, `TextStyle::with_color(color)`, `TextStyle::size() -> Option<u32>` and `TextStyle::color()`.
  - `Label::new(text: impl Into<String>, style: TextStyle, pos: TextPositioning)`, with `text()`, `style()` and `pos()`.
  - `TerminalCanvas::with_font(Font)` and `TerminalCanvas::with_font_size(u32)`.
- Produces (crate-private, used by Task 8):
  - `struct PlacedText { coverage: Coverage, color: RGB8, left: u32, top: u32 }`, where `top` is the canvas row of the top pixel and rows count up from the bottom.
  - `PlacedText::new(font, text, size, color, left, top)`, `PlacedText::from_label(font, &Label, size)`, `PlacedText::draw(&self, &mut Canvas)`, and `#[cfg(test)] PlacedText::bounds() -> Option<Limits<u32>>` (inclusive).
  - `Layout` with `tick_labels: Vec<PlacedText>` (x labels first, then y), `x_offset_label: Option<PlacedText>`, `y_offset_label: Option<PlacedText>`, and `fn into_texts(self)`.

- [ ] **Step 1: Write the failing tests**

In `src/plotting/text.rs`, replace the whole `#[cfg(test)] mod test { ... }` block with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::plotting::colors;

    #[test]
    fn text_style_sizes_are_clamped() {
        assert_eq!(TextStyle::new(colors::WHITE, 0).size(), Some(1));
        assert_eq!(TextStyle::new(colors::WHITE, 20).size(), Some(20));
        assert_eq!(TextStyle::new(colors::WHITE, 5000).size(), Some(MAX_FONT_SIZE));
        assert_eq!(TextStyle::with_color(colors::WHITE).size(), None);
    }
}
```

Add to the tests module in `src/plotting/canvas.rs` (the existing `white_axes()` helper is already there):

```rust
    #[test]
    fn labels_are_drawn_with_anti_aliased_text() {
        let label = Label::new(
            "0",
            TextStyle::new(colors::WHITE, 28),
            TextPositioning::Centered(Point::new(20, 20)),
        );
        let bytes = TerminalCanvas::new(40, 40, colors::BLACK)
            .with_label(label)
            .draw()
            .unwrap()
            .get_bytes();
        let reds: Vec<u8> = bytes.chunks(3).map(|px| px[0]).collect();
        assert!(reds.contains(&255), "fully covered pixels");
        assert!(reds.iter().any(|&v| v > 0 && v < 255), "partly covered edge pixels");
    }

    #[test]
    fn the_base_font_size_sets_the_tick_label_size() {
        let graph = Graph::new()
            .with_series(Series::new(&[Point::new(0.0, 0.0), Point::new(10.0, 10.0)]))
            .with_axes(white_axes());
        let layout_at = |size| {
            TerminalCanvas::new(800, 600, colors::BLACK)
                .with_font_size(size)
                .with_graph(graph.clone())
                .layout(&graph, &graph.view_limits().unwrap())
                .unwrap()
        };
        let (small, large) = (layout_at(14), layout_at(28));
        assert_eq!(small.tick_labels[0].coverage.height, Font::default().metrics(14).height());
        assert_eq!(large.tick_labels[0].coverage.height, Font::default().metrics(28).height());
        // bigger labels leave less room for the plot
        assert!(large.plot.span().1 < small.plot.span().1);
    }
```

Update the two existing layout tests in `canvas.rs` to the new `Layout` fields.

In `tick_labels_never_overlap`, replace the `x_labels` binding with:

```rust
            let x_labels: Vec<_> = (layout.tick_labels.iter())
                .take(layout.x_ticks.labels.len())
                .map(|l| l.bounds().unwrap())
                .collect();
```

In `offset_labels_stay_clear_of_other_labels`, replace everything from `let ticks = ...` to the end of the loop body with:

```rust
            let offsets: Vec<_> = (layout.x_offset_label.iter())
                .chain(&layout.y_offset_label)
                .map(|t| t.bounds().unwrap())
                .collect();
            assert_eq!(offsets.len(), 2, "{w}x{h}: offset labels");
            let ticks: Vec<_> = layout.tick_labels.iter().map(|t| t.bounds().unwrap()).collect();
            for offset in &offsets {
                assert!(offset.max().x < w && offset.max().y < h, "{w}x{h}: {offset:?}");
                for other in ticks.iter().chain(offsets.iter().filter(|o| *o != offset)) {
                    assert!(
                        !offset.intersects(other.clone()),
                        "{w}x{h}: {offset:?} overlaps {other:?}"
                    );
                }
            }
```

In `tests/properties.rs`, replace the `text_scale` and `text_padding` parameters with:

```rust
        text_size in prop_oneof![4 => 0u32..40, 1 => any::<u32>()],
        font_size in prop_oneof![4 => 1u32..40, 1 => any::<u32>()],
```

Then change `TextStyle::new(colors::WHITE, text_scale, text_padding)` to `TextStyle::new(colors::WHITE, text_size)`, and chain `.with_font_size(font_size)` after `TerminalCanvas::new(width, height, colors::BLACK)`.

- [ ] **Step 2: Run them to confirm they fail**

Run: `cargo test --lib`

Expected: the build fails. The errors will include "this function takes 3 arguments but 2 were supplied" for `TextStyle::new`, "no method named `with_font_size`", and "no field `tick_labels`".

- [ ] **Step 3: Rewrite `src/plotting/text.rs`**

Replace everything above the tests module with:

```rust
use super::point::Point;
use rgb::RGB8;

/// Base text size in pixels when none is set: matplotlib's default of 10 pt at 100 dpi.
pub const DEFAULT_FONT_SIZE: u32 = 14;

/// Largest text size in pixels; larger sizes are clamped to it.
pub const MAX_FONT_SIZE: u32 = 400;

/// Where a label is anchored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextPositioning {
    /// Centered on the point.
    Centered(Point<u32>),
    /// The left edge at the point, vertically centered.
    LeftAligned(Point<u32>),
}

impl TextPositioning {
    /// The same kind of positioning at another point.
    pub fn clone_with(&self, new_point: Point<u32>) -> Self {
        match self {
            Self::Centered(_) => Self::Centered(new_point),
            Self::LeftAligned(_) => Self::LeftAligned(new_point),
        }
    }

    /// The anchor point.
    pub fn point(&self) -> &Point<u32> {
        match self {
            Self::Centered(point) => point,
            Self::LeftAligned(point) => point,
        }
    }
}

/// Color and size of text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextStyle {
    color: RGB8,
    size: Option<u32>,
}

impl Default for TextStyle {
    /// Black text at the default size for where it is drawn.
    fn default() -> TextStyle {
        TextStyle::with_color(super::colors::BLACK)
    }
}

impl TextStyle {
    /// Text in `color` at `size` pixels (the em size, like CSS `font-size`), clamped to
    /// `1..=`[`MAX_FONT_SIZE`].
    pub fn new(color: RGB8, size: u32) -> TextStyle {
        TextStyle {
            color,
            size: Some(size.clamp(1, MAX_FONT_SIZE)),
        }
    }

    /// Text in `color` at the default size for where it is drawn: the canvas's base size
    /// ([`TerminalCanvas::with_font_size`](super::canvas::TerminalCanvas::with_font_size)).
    pub fn with_color(color: RGB8) -> TextStyle {
        TextStyle { color, size: None }
    }

    /// The text color.
    pub fn color(&self) -> RGB8 {
        self.color
    }

    /// The size in pixels, or `None` for the default size where the text is drawn.
    pub fn size(&self) -> Option<u32> {
        self.size
    }
}

/// Text placed on the canvas with
/// [`TerminalCanvas::with_label`](super::canvas::TerminalCanvas::with_label).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    text: String,
    style: TextStyle,
    pos: TextPositioning,
}

impl Label {
    /// `text` in `style`, placed at `pos`.
    pub fn new(text: impl Into<String>, style: TextStyle, pos: TextPositioning) -> Label {
        Label {
            text: text.into(),
            style,
            pos,
        }
    }

    /// The text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Its color and size.
    pub fn style(&self) -> &TextStyle {
        &self.style
    }

    /// Where it is placed.
    pub fn pos(&self) -> &TextPositioning {
        &self.pos
    }
}
```

Delete the bitmap font: `git rm src/plotting/numbers.rs`. In `src/plotting.rs`, remove `mod numbers;` and change the `text` module's doc comment to `/// Text styles and labels.`

- [ ] **Step 4: Switch the canvas to the font**

In `src/plotting/canvas.rs`:

1. Replace the `use super::{...}` block with the one below. Afterwards, delete any import the compiler reports as unused (`MaskPoints` and `TextStyle` probably will be). If you remove `TextStyle`, add `use crate::plotting::text::TextStyle;` to the tests module, because the new tests use it.

```rust
use super::{
    axes::{Axes, AxesPositioning},
    colors,
    common::{FloatConvertable, MaskPoints},
    font::{Coverage, Font},
    graph::Graph,
    limits::Limits,
    point::Point,
    srgb,
    text::{DEFAULT_FONT_SIZE, Label, MAX_FONT_SIZE, TextPositioning, TextStyle},
    ticks::{AxisTicks, axis_offset, fit_ticks, format_offset},
};
```

2. Replace the `Layout` struct with this struct plus `PlacedText`:

```rust
/// Where a graph is drawn on the canvas.
struct Layout {
    /// Area the view limits of the data are mapped onto.
    plot: Limits<u32>,
    /// Ticks relative to `x_offset`.
    x_ticks: AxisTicks,
    /// Ticks relative to `y_offset`.
    y_ticks: AxisTicks,
    /// Offset of the x axis ([`axis_offset`]), 0 for none.
    x_offset: f64,
    /// Offset of the y axis, 0 for none.
    y_offset: f64,
    /// Tick labels: x, then y.
    tick_labels: Vec<PlacedText>,
    x_offset_label: Option<PlacedText>,
    y_offset_label: Option<PlacedText>,
}

impl Layout {
    /// Every piece of text in the layout.
    fn into_texts(self) -> impl Iterator<Item = PlacedText> {
        (self.tick_labels.into_iter())
            .chain(self.x_offset_label)
            .chain(self.y_offset_label)
    }
}

/// Text rasterized and positioned on the canvas.
#[derive(Debug, Clone)]
struct PlacedText {
    coverage: Coverage,
    color: RGB8,
    /// Column of the leftmost pixel.
    left: u32,
    /// Row of the top pixel (rows count up from the bottom of the canvas).
    top: u32,
}

impl PlacedText {
    /// `text` rasterized at `size` pixels with its top-left pixel at (`left`, `top`).
    fn new(font: &Font, text: &str, size: u32, color: RGB8, left: u32, top: u32) -> PlacedText {
        PlacedText {
            coverage: font.rasterize(text, size),
            color,
            left,
            top,
        }
    }

    /// A label placed by its [`TextPositioning`]: centered on the point or starting at it, and
    /// vertically centered on it either way.
    fn from_label(font: &Font, label: &Label, size: u32) -> PlacedText {
        let coverage = font.rasterize(label.text(), size);
        let point = label.pos().point();
        let left = match label.pos() {
            TextPositioning::Centered(_) => point.x.saturating_sub(coverage.width / 2),
            TextPositioning::LeftAligned(_) => point.x,
        };
        let top = point.y.saturating_add(coverage.height / 2);
        PlacedText {
            coverage,
            color: label.style().color(),
            left,
            top,
        }
    }

    /// The pixels it covers (inclusive), or `None` when it has none.
    #[cfg(test)]
    fn bounds(&self) -> Option<Limits<u32>> {
        if self.coverage.width == 0 || self.coverage.height == 0 {
            return None;
        }
        Some(Limits::new(
            Point::new(self.left, self.top.saturating_sub(self.coverage.height - 1)),
            Point::new(self.left.saturating_add(self.coverage.width - 1), self.top),
        ))
    }

    fn draw(&self, canvas: &mut Canvas) {
        for row in 0..self.coverage.height {
            let Some(y) = self.top.checked_sub(row) else {
                break;
            };
            for col in 0..self.coverage.width {
                let coverage = self.coverage.get(col, row);
                if coverage > 0 {
                    canvas.blend(self.left.saturating_add(col), y, self.color, coverage);
                }
            }
        }
    }
}
```

3. Add `font: Font` and `font_size: u32` to `TerminalCanvas` after `labels`. Initialize them in `new` as `font: Font::default(), font_size: DEFAULT_FONT_SIZE,`, and add these builders after `with_label`:

```rust
    /// Sets the font for all text. Characters it lacks are drawn with the built-in Go font.
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Sets the base text size in pixels (the em size), clamped to `1..=`[`MAX_FONT_SIZE`].
    /// Tick labels and any text without a size of its own use it. The default is
    /// [`DEFAULT_FONT_SIZE`].
    pub fn with_font_size(mut self, px: u32) -> Self {
        self.font_size = px.clamp(1, MAX_FONT_SIZE);
        self
    }
```

4. In `draw`, replace `self.labels.extend(layout.labels);` and the whole "labels are drawn last" block. Declare `let mut texts = Vec::new();` before `if let Some(graph) = self.graph.take() {`, push the layout's text with `texts.extend(layout.into_texts());` inside the block, and after the block add:

```rust
        // user labels, then all text is drawn last so the graph doesn't cover it
        for label in &self.labels {
            let size = label.style().size().unwrap_or(self.font_size);
            texts.push(PlacedText::from_label(&self.font, label, size));
        }
        for text in &texts {
            text.draw(&mut self.canvas);
        }
```

5. Replace `label_style` with:

```rust
    /// Color and size of tick labels: the axes' text style, at the canvas's base size when it
    /// has no size of its own. A label color identical to the background (e.g. the black default
    /// text on the default black canvas) is replaced with black or white, whichever contrasts
    /// with the background.
    fn label_style(&self, axes: Option<&Axes>) -> (RGB8, u32) {
        let style = axes.map(|a| *a.style()).unwrap_or_default();
        let color = if style.color() == self.background {
            let luminance = 0.2126 * self.background.r as f64
                + 0.7152 * self.background.g as f64
                + 0.0722 * self.background.b as f64;
            if luminance > 127.5 {
                colors::BLACK
            } else {
                colors::WHITE
            }
        } else {
            style.color()
        };
        (color, style.size().unwrap_or(self.font_size))
    }
```

6. In `layout`, measure and place text with the font. Replace the three lines from `let style = self.label_style(axes.as_ref());` through `let text_h = text("0").height() as u32;` with:

```rust
        let (color, size) = self.label_style(axes.as_ref());
        let font = &self.font;
        let width = |text: &str| font.width(text, size).ceil() as u32;
        let place =
            |text: &str, left: u32, top: u32| PlacedText::new(font, text, size, color, left, top);
        let text_h = font.metrics(size).height();
```

Then change every `text(l).width() as u32` to `width(l)` (there are two: in `y_label_w` and in the x-tick `widths` closure). Finally, replace everything from `let plot = Limits::new(plot_min, plot_max);` to the end of the function with:

```rust
        let plot = Limits::new(plot_min, plot_max);
        let mut tick_labels = Vec::new();
        if show_x_labels {
            // the top line of the band
            let top = outer_min.y + x_band - 1;
            for (&value, label) in x_ticks.values.iter().zip(&x_ticks.labels) {
                let w = width(label);
                tick_labels.push(place(label, x_label_center(value, w) - w / 2, top));
            }
        }
        if show_y_labels {
            for (&value, label) in y_ticks.values.iter().zip(&y_ticks.labels) {
                // right-aligned against the plot area, vertically centered on the tick
                let left = outer_min.x + y_label_w - width(label);
                tick_labels.push(place(label, left, y_label_center(value) + text_h / 2));
            }
        }

        // where matplotlib puts them: the x offset under the right end of the x axis, the y
        // offset above the top of the y axis
        let x_offset_label = x_offset_label.map(|text| {
            let left = (plot_max_x + 1).saturating_sub(width(&text));
            place(&text, left, outer_min.y + text_h - 1)
        });
        let y_offset_label = y_offset_label
            .map(|text| place(&text, plot_min_x.saturating_sub(axes_inset.0), outer_max.y));

        Ok(Layout {
            plot,
            x_ticks,
            y_ticks,
            x_offset,
            y_offset,
            tick_labels,
            x_offset_label,
            y_offset_label,
        })
```

In `README.md`, replace the features bullet `- **Bitmap text** — built-in 10x11 pixel font for labels and numeric annotations` with:

```markdown
- **TrueType text** — an embedded, anti-aliased Go font covering Latin-1, Greek and common math symbols; load your own font for other scripts
```

- [ ] **Step 5: Run the unit tests to confirm they pass**

Run: `cargo test --lib`

Expected: everything passes, including the two new tests and the updated layout tests.

- [ ] **Step 6: Regenerate and review the images**

Run: `TERMPLT_UPDATE_SNAPSHOTS=1 cargo test --test golden && scripts/readme_images.sh`

Open every file under `tests/snapshots/` and `docs/images/`. Check that:
- tick labels are smooth, readable Go digits;
- no label overlaps another or leaves the canvas;
- the white-on-white scene has black labels;
- the offset scene shows `+1e15` and `+1.7e9`.

Then run `cargo test --test golden` again to confirm the snapshots match.

- [ ] **Step 7: Run the full gate and commit**

Run the full gate. From this task on, clippy runs without `-A dead_code`.

```bash
git add -A src tests README.md docs/images
git commit -F - <<'EOF'
Draw all text with the TrueType font

The bitmap digit font is gone. TextStyle takes a size in pixels, Label holds its text,
TerminalCanvas has with_font and with_font_size (14 px by default), and every label is
rasterized with ab_glyph and blended in linear light. Snapshots and README images are
regenerated.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 5: Use the Unicode minus sign in numbers

**Files:**
- Modify: `src/plotting/ticks.rs`
- Regenerate: `tests/snapshots/*.png`, `docs/images/*.png`

**Interfaces:**
- Consumes: nothing new.
- Produces: `format_ticks` and `format_offset` return strings using `−` (U+2212) for negative values and negative exponents.

- [ ] **Step 1: Update the expected strings in the existing tests** (these are the failing tests)

In `src/plotting/ticks.rs` tests:
- `labels_share_decimal_places`: `["-0.25", "0", "0.25"]` becomes `["−0.25", "0", "0.25"]`.
- `large_and_tiny_values_use_scientific_notation`: `["1e-5", "2e-5"]` becomes `["1e−5", "2e−5"]`.
- `offset_labels`: `Some("-1e15")` becomes `Some("−1e15")`, and `Some("+1.25e-3")` becomes `Some("+1.25e−3")`.

Leave `negative_zero_is_normalized` unchanged: `normalize_negative_zero` still works on the hyphen before the swap.

- [ ] **Step 2: Run them to confirm they fail**

Run: `cargo test --lib plotting::ticks`

Expected: 3 failures, each showing a `-` where `−` was expected.

- [ ] **Step 3: Implement**

Add near the top of `ticks.rs`:

```rust
/// The minus sign in labels: U+2212, as matplotlib uses by default (`axes.unicode_minus`). It
/// is as wide as `+` and sits at its height, unlike the shorter hyphen.
const MINUS: &str = "\u{2212}";
```

In `format_all`, change `normalize_negative_zero(format(v))` to `normalize_negative_zero(format(v)).replace('-', MINUS)`. In `format_offset`, change `let label = format!("{offset:e}");` to `let label = format!("{offset:e}").replace('-', MINUS);`.

- [ ] **Step 4: Run the tests, regenerate and review the images**

Run: `cargo test --lib && TERMPLT_UPDATE_SNAPSHOTS=1 cargo test --test golden && scripts/readme_images.sh`

Open the changed images. Negative tick labels should show a minus as wide as `+`.

- [ ] **Step 5: Run the gate and commit**

```bash
git add src/plotting/ticks.rs tests/snapshots docs/images
git commit -F - <<'EOF'
Use the Unicode minus sign in tick and offset labels

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 6: Ship the licenses, bump to 0.3.0 and open PR 1

**Files:**
- Modify: `.github/workflows/release.yml`, `Cargo.toml` (version), `CHANGELOG.md`, `README.md` (License and Architecture), `CLAUDE.md`

**Interfaces:** none (release and docs).

- [ ] **Step 1: Put the license files in the release archives**

In `.github/workflows/release.yml`, change the `run` of "Package binary (Unix)" to:

```yaml
        run: |
          ARCHIVE="${{ env.BINARY_NAME }}-${{ env.TAG }}-${{ matrix.target }}.${{ matrix.archive_ext }}"
          # the licenses travel with the binary: termplt's (MIT) and the embedded Go font's (BSD-3)
          tar czf "$ARCHIVE" -C "target/${{ matrix.target }}/release" ${{ env.BINARY_NAME }} \
            -C "$GITHUB_WORKSPACE" LICENSE -C "$GITHUB_WORKSPACE/assets/fonts" LICENSE-Go
          echo "ARCHIVE=$ARCHIVE" >> "$GITHUB_ENV"
```

and the `Compress-Archive` line of "Package binary (Windows)" to:

```powershell
          Compress-Archive -Path "target/${{ matrix.target }}/release/${{ env.BINARY_NAME }}.exe", "LICENSE", "assets/fonts/LICENSE-Go" -DestinationPath $ARCHIVE
```

- [ ] **Step 2: Check the tar command locally**

Run:

```bash
cargo build --release --locked
tmp=$(mktemp -d)
GITHUB_WORKSPACE=$PWD tar czf "$tmp/t.tar.gz" -C target/release termplt \
  -C "$PWD" LICENSE -C "$PWD/assets/fonts" LICENSE-Go
tar tzf "$tmp/t.tar.gz"; ls -l "$tmp/t.tar.gz"
```

Expected: the listing is `termplt`, `LICENSE`, `LICENSE-Go`. Note the archive size and compare it with v0.2.1's `termplt-v0.2.1-aarch64-apple-darwin.tar.gz`, 815,099 bytes. This is an approximation: the local toolchain may differ from CI's.

- [ ] **Step 3: Version, changelog and docs**

- **Version:** set `version = "0.3.0"` in `Cargo.toml`, then run `cargo build` to update `Cargo.lock`.
- **Changelog:** add above `## [0.2.1]` in `CHANGELOG.md`:

```markdown
## [0.3.0] - Unreleased

Text is drawn with an embedded TrueType font. The low-level text API has **breaking changes**;
see "Changed" and "Removed".

### Added
- `plotting::font::Font` (also in the prelude): the built-in Go font by default, or any
  TrueType or OpenType font with `Font::from_bytes`. Characters a font lacks are drawn with Go.
- `TerminalCanvas::with_font` and `with_font_size`, `text::DEFAULT_FONT_SIZE` (14 px) and
  `text::MAX_FONT_SIZE` (400 px).
- `Error::InvalidFont`.
- Release archives include `LICENSE` and the font's `LICENSE-Go`.

### Changed
- All text, tick labels included, is drawn with an embedded, trimmed copy of the Go font
  (BSD-3-Clause), anti-aliased and blended in linear light. It covers Latin-1, Greek and common
  math symbols. The crate's license is now `MIT AND BSD-3-Clause`.
- Numbers use the Unicode minus sign `−`, as matplotlib does.
- **Breaking:** `TextStyle::new(color, size_px)` replaces `TextStyle::new(color, scale,
  padding)`, and `TextStyle::size()` replaces `scale()` and `padding()`.
  `TextStyle::with_color` is unchanged; its text takes the canvas's base size.
- **Breaking:** `Label::new(text, style, pos)` with `text()`, `style()` and `pos()` replaces
  `Label::new(Text, TextPositioning)`, `txt()` and `limits()`. `Label` no longer implements
  `Drawable`.

### Removed
- **Breaking:** `Text`, `MAX_TEXT_SCALE` and `MAX_TEXT_PADDING` (the bitmap font).
```

  and, above the `[0.2.1]` link at the bottom:

```markdown
[0.3.0]: https://github.com/EdCarney/termplt/compare/v0.2.1...HEAD
```

- **README:** change the License section to:

```markdown
This project is licensed under the [MIT License](https://github.com/EdCarney/termplt/blob/main/LICENSE). The embedded Go font is © 2016 Bigelow & Holmes Inc. and licensed under the [BSD 3-Clause License](https://github.com/EdCarney/termplt/blob/main/assets/fonts/LICENSE-Go).
```

  and add this row to the README's Architecture table after `plotting::canvas`:

```markdown
| `plotting::font` | `Font`: the built-in Go font or your own; text is rasterized with `ab_glyph` |
```

- **CLAUDE.md:**
  - Add `scripts/subset_font.sh                # Rebuild the embedded font from the pinned Go source (needs python3 and network)` to the build commands.
  - Change `plotting::{common, numbers, ticks}` to `plotting::{common, srgb, ticks}`.
  - In the pipeline diagram, replace the last line with `  └── text → font.rasterize → Canvas::blend  # all labels, drawn last`.
  - Replace the whole "Text/Number Rendering" subsection with:

```markdown
### Text Rendering (`font.rs`, `srgb.rs`, `text.rs`)

Text uses an embedded Go Regular font, trimmed to `assets/fonts/charset.txt` by `scripts/subset_font.sh` (pinned source, hash-checked; a unit test checks the font covers the charset). `Font` (the built-in font or `Font::from_bytes`) falls back to Go for missing characters, then to the `.notdef` box. `Font::rasterize` draws one line into an 8-bit `Coverage` bitmap with `ab_glyph`; the canvas's `PlacedText` positions it and `Canvas::blend` mixes it in linear light using the committed integer table in `srgb.rs` (no `powf`, so every OS gives the same bytes). Sizes are em sizes in whole pixels: `TextStyle` has an optional size (none = the canvas's base size, `DEFAULT_FONT_SIZE` = 14). Numbers use the Unicode minus.
```

  - Replace the Known Issues bullet about the bitmap font with `- No titles, axis names or legends yet (in progress: see \`docs/superpowers/specs/2026-09-25-text-rendering-design.md\`)`.

- [ ] **Step 4: Run the full gate and commit**

```bash
git add .github/workflows/release.yml Cargo.toml Cargo.lock CHANGELOG.md README.md CLAUDE.md
git commit -F - <<'EOF'
Ship the font license with releases and start 0.3.0

Release archives now include LICENSE and the Go font's LICENSE-Go, which BSD-3 requires and
which also fixes MIT's notice being missing from 0.2.x archives.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

- [ ] **Step 5: Push and open PR 1**

```bash
git push -u origin text-engine
gh pr create --base main --title "Draw text with an embedded TrueType font (0.3.0, part 1)" --body "$(cat <<'EOF'
## Summary
Part 1 of spec 1 (`docs/superpowers/specs/2026-09-25-text-rendering-design.md`; the spec and
its implementation plan land with this PR). No new features yet: every label is now drawn with
an embedded, anti-aliased Go font instead of the bitmap digits.

- `plotting::font::Font`: the built-in Go subset (Latin-1, Greek, math symbols) or a user
  font, with fallback to Go
- linear-light blending with an integer sRGB table, so output is identical on every OS
- breaking: `TextStyle::new(color, size_px)`, `Label::new(text, style, pos)`; `Text` removed
- Unicode minus in numbers; licenses in release archives; crate license `MIT AND BSD-3-Clause`
- archive size (local aarch64 build): <before> → <after> bytes

## Test plan
- [x] font coverage of `charset.txt`, fallback, metrics, rasterizing, blending (hand-derived values)
- [x] all golden snapshots and README images regenerated and reviewed
- [x] fmt, clippy, tests (with and without default features), docs
- [ ] MSRV 1.88 in CI; golden images identical on Linux, macOS and Windows

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

Replace `<before>` and `<after>` with the sizes from Step 2 before running. Then watch CI with `gh pr checks --watch`.

- **A golden test fails on another OS:** that means the anti-aliased pixels differ between platforms. Stop and report it. The fallback in spec § Testing (allow ±1 color level per pixel) needs the user's go-ahead.

---

# PR 2: Titles and axis names

```bash
# once PR 1 is merged; if it isn't, branch from text-engine and pass --base text-engine to gh
git checkout main && git pull --ff-only && git checkout -b titles-and-axis-names
```

### Task 7: `wrap()` for titles and axis names

**Files:**
- Modify: `src/plotting/text.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub(crate) const ELLIPSIS: &str = "…"`.
  - `pub(crate) fn wrap(text: &str, max_width: f32, max_lines: usize, measure: impl Fn(&str) -> f32) -> Vec<String>`. It breaks lines at spaces (runs of spaces collapse) and at `\n`, and keeps at most `max_lines` lines. When text is cut, the last line kept ends with `…`, and so does any single word wider than `max_width`. It returns an empty list for blank text, `max_lines == 0`, or when not even `…` fits.

- [ ] **Step 1: Write the failing tests** (add to the tests module in `text.rs`)

```rust
    /// Every character is 10 px wide.
    fn fixed(s: &str) -> f32 {
        s.chars().count() as f32 * 10.0
    }

    #[test]
    fn short_text_stays_on_one_line() {
        assert_eq!(wrap("Oscillator response", 1000.0, 3, fixed), ["Oscillator response"]);
    }

    #[test]
    fn lines_break_at_spaces() {
        assert_eq!(wrap("one two three", 80.0, 3, fixed), ["one two", "three"]);
        // runs of spaces collapse
        assert_eq!(wrap("one   two", 80.0, 3, fixed), ["one two"]);
    }

    #[test]
    fn newlines_force_a_break() {
        assert_eq!(wrap("Time\n(s)", 1000.0, 3, fixed), ["Time", "(s)"]);
    }

    #[test]
    fn text_past_the_line_limit_ends_with_an_ellipsis() {
        // "a b" / "c d" / "e" doesn't fit in two lines, so the second ends with "…"; "c d…" is
        // too wide, so it loses the "d"
        assert_eq!(wrap("a b c d e", 30.0, 2, fixed), ["a b", "c…"]);
    }

    #[test]
    fn a_word_too_wide_for_a_line_is_cut() {
        assert_eq!(wrap("abcdefghij", 50.0, 2, fixed), ["abcd…"]);
        // cutting respects multi-byte characters
        assert_eq!(wrap("ΔΔΔΔΔΔ", 30.0, 1, fixed), ["ΔΔ…"]);
    }

    #[test]
    fn nothing_is_returned_when_nothing_fits() {
        assert!(wrap("anything", 5.0, 3, fixed).is_empty());
        assert!(wrap("   ", 1000.0, 3, fixed).is_empty());
        assert!(wrap("text", 1000.0, 0, fixed).is_empty());
    }
```

- [ ] **Step 2: Run them to confirm they fail**

Run: `cargo test --lib plotting::text`

Expected: the build fails with "cannot find function `wrap`".

- [ ] **Step 3: Implement** (in `text.rs`, above the tests)

```rust
/// Marks text cut to fit.
pub(crate) const ELLIPSIS: &str = "…";

/// Breaks `text` into at most `max_lines` lines no wider than `max_width`, as measured by
/// `measure`. Lines break at spaces (runs of spaces collapse) and at `\n`. When the text doesn't
/// fit, the last line kept ends with [`ELLIPSIS`]; a single word wider than `max_width` is cut
/// the same way rather than broken. Returns no lines for blank text, or when not even the
/// ellipsis fits.
pub(crate) fn wrap(
    text: &str,
    max_width: f32,
    max_lines: usize,
    measure: impl Fn(&str) -> f32,
) -> Vec<String> {
    if max_lines == 0 || measure(ELLIPSIS) > max_width {
        return Vec::new();
    }
    // greedy breaking: a word joins the current line if the line still fits
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let joined = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if line.is_empty() || measure(&joined) <= max_width {
                line = joined;
            } else {
                lines.push(std::mem::replace(&mut line, word.to_string()));
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    let cut = lines.len() > max_lines;
    lines.truncate(max_lines);
    let last = lines.len().saturating_sub(1);
    for (i, line) in lines.iter_mut().enumerate() {
        if measure(line) > max_width || (cut && i == last) {
            *line = with_ellipsis(line, max_width, &measure);
        }
    }
    lines
}

/// The longest start of `line` that fits in `max_width` with an ellipsis after it.
fn with_ellipsis(line: &str, max_width: f32, measure: &impl Fn(&str) -> f32) -> String {
    let mut end = line.len();
    loop {
        let candidate = format!("{}{ELLIPSIS}", line[..end].trim_end());
        if end == 0 || measure(&candidate) <= max_width {
            return candidate;
        }
        // drop the last character
        end = line[..end].char_indices().next_back().map_or(0, |(i, _)| i);
    }
}
```

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `cargo test --lib plotting::text`

Expected: 7 tests pass.

- [ ] **Step 5: Run the gate** (clippy with `-A dead_code`: the layout in Task 8 is the first caller) **and commit**

```bash
git add src/plotting/text.rs
git commit -F - <<'EOF'
Add wrap() for titles and axis names

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 8: Layout bands with a title and axis names

This is the layout from spec § Layout:
- bands whose gaps scale with the text size;
- y tick labels centered on their digits;
- a wrapped title, x name and y name, with the y name turned to read upwards;
- two layout passes, so shared lines that would collide get stacked.

**Files:**
- Modify: `src/plotting/graph.rs`, `src/plotting/font.rs`, `src/plotting/canvas.rs`
- Regenerate: `tests/snapshots/*.png`, `docs/images/*.png`

**Interfaces:**
- Consumes:
  - `wrap` and `ELLIPSIS` from Task 7;
  - `Font::{metrics, width, rasterize}` and `LineMetrics::{ascent, digit_height, height}`;
  - `PlacedText::new` and `PlacedText::bounds`;
  - `axis_offset` and `format_offset`.
- Produces:
  - `Graph::with_title`, `with_x_label`, `with_y_label` (each `impl Into<String>`) and the getters `title()`, `x_label()`, `y_label()` returning `Option<&str>`.
  - `Coverage::rotated_ccw(&self) -> Coverage`.
  - `Layout` gains `title`, `x_name` and `y_name`, each a `Vec<PlacedText>`.
  - `PlacedText::bounds` is no longer test-only.

- [ ] **Step 1: Add the `Graph` fields**

In `src/plotting/graph.rs`, add to `struct Graph` after `grid_lines`:

```rust
    title: Option<String>,
    x_label: Option<String>,
    y_label: Option<String>,
```

add `title: self.title.clone(), x_label: self.x_label.clone(), y_label: self.y_label.clone(),` to the struct literal in `clone_without_data`, and add to `impl Graph` after `with_grid_lines`:

```rust
    /// Draws `text` as a title above the plot, 1.2 times the size of the tick labels. Long
    /// titles wrap onto up to 3 lines; text that still doesn't fit ends with `…`.
    pub fn with_title(mut self, text: impl Into<String>) -> Self {
        self.title = Some(text.into());
        self
    }

    /// Names the x axis: `text` is centered below the tick labels, wrapping onto up to 2 lines.
    /// An axis without tick labels (see [`Axes`]) shows no name.
    pub fn with_x_label(mut self, text: impl Into<String>) -> Self {
        self.x_label = Some(text.into());
        self
    }

    /// Names the y axis: `text` is drawn left of the tick labels, turned to read upwards and
    /// wrapping onto up to 2 lines. An axis without tick labels shows no name.
    pub fn with_y_label(mut self, text: impl Into<String>) -> Self {
        self.y_label = Some(text.into());
        self
    }
```

and after the `grid_lines()` getter:

```rust
    /// The title, if one was set.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// The x axis name, if one was set.
    pub fn x_label(&self) -> Option<&str> {
        self.x_label.as_deref()
    }

    /// The y axis name, if one was set.
    pub fn y_label(&self) -> Option<&str> {
        self.y_label.as_deref()
    }
```

- [ ] **Step 2: Write the failing tests**

In `src/plotting/font.rs` tests:

```rust
    #[test]
    fn rotation_turns_the_bitmap_counter_clockwise() {
        // rows [1, 2, 3] and [4, 5, 6]: the left column ends up along the bottom, so a line of
        // text reads from bottom to top
        let line = Coverage {
            width: 3,
            height: 2,
            data: vec![1, 2, 3, 4, 5, 6],
        };
        let turned = line.rotated_ccw();
        assert_eq!((turned.width, turned.height), (2, 3));
        assert_eq!(turned.data, vec![3, 6, 2, 5, 1, 4]);
    }
```

In `src/plotting/canvas.rs` tests, add the helpers and tests below. Add `use crate::plotting::marker::MarkerStyle;` to the tests module if it isn't imported yet.

```rust
    /// A series that draws nothing itself, so the plot's inset is just the 2-row axis.
    fn plain_series() -> Series {
        Series::new(&[Point::new(0.0, 0.0), Point::new(10.0, 10.0)])
            .with_marker_style(MarkerStyle::None)
    }

    fn plain_graph() -> Graph {
        Graph::new().with_series(plain_series()).with_axes(white_axes())
    }

    fn layout_of(graph: &Graph, (w, h): (u32, u32), font_size: u32) -> Layout {
        TerminalCanvas::new(w, h, colors::BLACK)
            .with_buffer(BufferType::Uniform(8))
            .with_font_size(font_size)
            .with_graph(graph.clone())
            .layout(graph, &graph.view_limits().unwrap())
            .unwrap()
    }

    /// Values near 1e15: the y axis gets an offset label above its top-left end.
    fn near_1e15() -> Vec<Point<f64>> {
        (0..=10).map(|i| Point::new(i as f64, 1e15 + 0.1 * i as f64)).collect()
    }

    /// Timestamps: the x axis gets an offset label under its right end.
    fn timestamps() -> Vec<Point<f64>> {
        (0..=10).map(|i| Point::new(1_700_000_000.0 + 10.0 * i as f64, i as f64)).collect()
    }

    /// Words that fill most of `width` pixels at `size`.
    fn line_filling(width: u32, size: u32) -> String {
        let font = Font::default();
        let mut text = String::from("Wide");
        while font.width(&format!("{text} title"), size) < width as f32 - 30.0 {
            text.push_str(" title");
        }
        text
    }

    #[test]
    fn tick_labels_sit_a_third_of_an_em_below_the_plot() {
        let layout = layout_of(&plain_graph(), (800, 600), 20);
        // 0.35 em at 20 px is 7 rows, below the 2-row axis inset
        assert_eq!(layout.tick_labels[0].top, layout.plot.min().y - 2 - 7 - 1);
    }

    #[test]
    fn y_tick_labels_are_centered_on_their_tick_by_the_digits() {
        let graph = plain_graph();
        let view = graph.view_limits().unwrap();
        let layout = layout_of(&graph, (800, 600), 28);
        let x_count = layout.x_ticks.labels.len();
        let (plot_min, plot_max) = (layout.plot.min().y as f64, layout.plot.max().y as f64);
        for (label, &value) in layout.tick_labels[x_count..].iter().zip(&layout.y_ticks.values) {
            let c = &label.coverage;
            let inked: Vec<u32> = (0..c.height)
                .filter(|&row| (0..c.width).any(|col| c.get(col, row) > 0))
                .collect();
            let ink_middle = label.top as f64 - f64::from(inked[0] + inked[inked.len() - 1]) / 2.0;
            let tick = to_canvas(value, view.min().y, view.max().y, plot_min, plot_max);
            assert!(
                (ink_middle - tick).abs() <= 1.0,
                "label for {value}: ink centered at {ink_middle}, tick at {tick}"
            );
        }
    }

    #[test]
    fn a_title_and_axis_names_take_room_from_the_plot() {
        let base = layout_of(&plain_graph(), (800, 600), 14).plot;
        let titled = layout_of(&plain_graph().with_title("Title"), (800, 600), 14);
        assert_eq!(titled.title.len(), 1);
        assert!(titled.plot.max().y < base.max().y);
        let x_named = layout_of(&plain_graph().with_x_label("Time (s)"), (800, 600), 14);
        assert_eq!(x_named.x_name.len(), 1);
        assert!(x_named.plot.min().y > base.min().y);
        let y_named = layout_of(&plain_graph().with_y_label("Amplitude"), (800, 600), 14);
        assert_eq!(y_named.y_name.len(), 1);
        assert!(y_named.plot.min().x > base.min().x);
    }

    #[test]
    fn blank_text_takes_no_room() {
        let blank = plain_graph()
            .with_title("  ")
            .with_x_label("")
            .with_y_label(" \n ");
        let layout = layout_of(&blank, (800, 600), 14);
        assert!(layout.title.is_empty() && layout.x_name.is_empty() && layout.y_name.is_empty());
        assert_eq!(layout.plot, layout_of(&plain_graph(), (800, 600), 14).plot);
    }

    #[test]
    fn a_long_title_wraps_to_three_lines_inside_the_buffer() {
        let words = "the quick brown fox jumps over the lazy dog ".repeat(12);
        let layout = layout_of(&plain_graph().with_title(words), (400, 300), 14);
        assert_eq!(layout.title.len(), 3);
        for line in &layout.title {
            let b = line.bounds().unwrap();
            // the buffered area of a 400-pixel canvas with an 8-pixel buffer is columns 8-391
            assert!(b.min().x >= 8 && b.max().x <= 391, "{b:?} leaves the buffered area");
        }
    }

    #[test]
    fn the_y_name_reads_upwards_beside_the_tick_labels() {
        let layout = layout_of(&plain_graph().with_y_label("Amplitude (µV)"), (800, 600), 14);
        let name = layout.y_name[0].bounds().unwrap();
        let (w, h) = name.span();
        assert!(h > w, "turned upright: {name:?}");
        let name_middle = (name.min().y + name.max().y) / 2;
        let plot_middle = (layout.plot.min().y + layout.plot.max().y) / 2;
        assert!(name_middle.abs_diff(plot_middle) <= 1);
        let first_y_label = layout.tick_labels[layout.x_ticks.labels.len()].bounds().unwrap();
        assert!(name.max().x < first_y_label.min().x);
    }

    #[test]
    fn the_title_shares_the_y_offset_line_until_they_would_meet() {
        let graph = Graph::new()
            .with_series(Series::new(&near_1e15()))
            .with_axes(white_axes());
        let short = layout_of(&graph.clone().with_title("T"), (800, 600), 14);
        let offset = short.y_offset_label.as_ref().unwrap().bounds().unwrap();
        let title = short.title[0].bounds().unwrap();
        assert!(title.min().y <= offset.max().y, "one band: {title:?} {offset:?}");

        // a title almost as wide as the canvas reaches the offset at the left and moves up
        let wide = layout_of(&graph.with_title(line_filling(784, 17)), (800, 600), 14);
        assert_eq!(wide.title.len(), 1);
        let offset = wide.y_offset_label.as_ref().unwrap().bounds().unwrap();
        let title = wide.title[0].bounds().unwrap();
        assert!(title.min().y > offset.max().y, "stacked: {title:?} above {offset:?}");
    }

    #[test]
    fn the_x_offset_shares_the_x_name_line_until_they_would_meet() {
        let graph = Graph::new()
            .with_series(Series::new(&timestamps()))
            .with_axes(white_axes());
        let short = layout_of(&graph.clone().with_x_label("t"), (800, 600), 14);
        assert_eq!(short.x_offset_label.as_ref().unwrap().top, short.x_name[0].top);

        let wide = layout_of(&graph.with_x_label(line_filling(784, 14)), (800, 600), 14);
        let offset = wide.x_offset_label.as_ref().unwrap().bounds().unwrap();
        let name = wide.x_name.last().unwrap().bounds().unwrap();
        assert!(offset.max().y < name.min().y, "stacked: {offset:?} below {name:?}");
    }

    #[test]
    fn no_text_overlaps_other_text() {
        let points: Vec<_> = (0..=10)
            .map(|i| Point::new(1_700_000_000.0 + 10.0 * i as f64, 1e15 + 0.1 * i as f64))
            .collect();
        let graph = Graph::new()
            .with_series(Series::new(&points))
            .with_axes(white_axes())
            .with_title("Sensor drift")
            .with_x_label("Time (s)")
            .with_y_label("Reading (µV)");
        for (w, h) in [(800, 600), (500, 300), (300, 200)] {
            let layout = layout_of(&graph, (w, h), 14);
            assert!(layout.x_offset_label.is_some() && layout.y_offset_label.is_some());
            let boxes: Vec<Limits<u32>> = layout.into_texts().filter_map(|t| t.bounds()).collect();
            for (i, a) in boxes.iter().enumerate() {
                assert!(a.max().x < w && a.max().y < h, "{w}x{h}: {a:?} leaves the canvas");
                for b in &boxes[i + 1..] {
                    assert!(!a.intersects(b.clone()), "{w}x{h}: {a:?} overlaps {b:?}");
                }
            }
        }
    }

    #[test]
    fn an_axis_without_tick_labels_has_no_name() {
        use crate::plotting::line::LineStyle;
        let y_only = Axes::new(
            AxesPositioning::YOnly(LineStyle::solid(colors::WHITE, 1)),
            TextStyle::with_color(colors::WHITE),
        );
        let graph = Graph::new()
            .with_series(plain_series())
            .with_axes(y_only)
            .with_x_label("x")
            .with_y_label("y");
        let layout = layout_of(&graph, (800, 600), 14);
        assert!(layout.x_name.is_empty());
        assert_eq!(layout.y_name.len(), 1);
    }
```

- [ ] **Step 3: Run them to confirm they fail**

Run: `cargo test --lib`

Expected: the build fails with "no method named `rotated_ccw`" and "no field `title` on type `Layout`".

- [ ] **Step 4: Add the rotation** (in `impl Coverage` in `font.rs`)

```rust
    /// Turned 90° counter-clockwise, so a line of text reads from bottom to top.
    pub fn rotated_ccw(&self) -> Coverage {
        let (width, height) = (self.height, self.width);
        let mut data = vec![0; self.data.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                // (x, y) moves to column y, row (old width - 1 - x)
                let i = (self.width - 1 - x) as usize * width as usize + y as usize;
                data[i] = self.get(x, y);
            }
        }
        Coverage {
            width,
            height,
            data,
        }
    }
```

- [ ] **Step 5: Rewrite the layout** (in `src/plotting/canvas.rs`)

1. Import `wrap` (`text::{DEFAULT_FONT_SIZE, Label, MAX_FONT_SIZE, TextPositioning, wrap}`, keeping whatever else that list already has). Replace the `LABEL_GAP` constant with:

```rust
/// Most lines a title wraps onto.
const TITLE_MAX_LINES: usize = 3;
/// Most lines an axis name wraps onto.
const NAME_MAX_LINES: usize = 2;
/// Title size relative to the tick labels (matplotlib's `large`).
const TITLE_SCALE: f32 = 1.2;
/// Baseline-to-baseline distance of wrapped lines, in em (matplotlib's `linespacing`).
const LINE_SPACING: f32 = 1.2;
/// Gap between the tick labels and the plot, in em (matplotlib's `xtick.major.pad`).
const TICK_GAP: f32 = 0.35;
/// Gap between an axis name and the tick labels, in em (matplotlib's `axes.labelpad`).
const NAME_GAP: f32 = 0.4;
/// Gap between the title and the plot, in em (matplotlib's `axes.titlepad`).
const TITLE_GAP: f32 = 0.6;
/// Gap around an offset label, in em (matplotlib's `Axis.OFFSETTEXTPAD`).
const OFFSET_GAP: f32 = 0.3;
/// Text sharing a line and closer than this, in em, is moved onto separate lines.
const MIN_APART: f32 = 0.5;
```

2. Add three fields to `Layout` after `y_offset_label`, and extend `into_texts`:

```rust
    /// Title lines, first on top.
    title: Vec<PlacedText>,
    /// x axis name lines, first on top.
    x_name: Vec<PlacedText>,
    /// y axis name lines turned to read upwards, first on the left.
    y_name: Vec<PlacedText>,
```

```rust
    fn into_texts(self) -> impl Iterator<Item = PlacedText> {
        (self.tick_labels.into_iter())
            .chain(self.x_offset_label)
            .chain(self.y_offset_label)
            .chain(self.title)
            .chain(self.x_name)
            .chain(self.y_name)
    }
```

3. Remove `#[cfg(test)]` from `PlacedText::bounds`, and add after `check_area`:

```rust
/// Which text that normally shares a line gets a line of its own: the title (above the y
/// offset) and the x offset (below the x axis name).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Stacking {
    title: bool,
    x_offset: bool,
}

/// Columns of empty space between two boxes side by side; 0 when their columns overlap.
fn horizontal_gap(a: &Limits<u32>, b: &Limits<u32>) -> u32 {
    if a.max().x < b.min().x {
        b.min().x - a.max().x - 1
    } else if b.max().x < a.min().x {
        a.min().x - b.max().x - 1
    } else {
        0
    }
}
```

4. Replace the whole `fn layout` with these two functions:

```rust
    /// Lays out the graph. Text that normally shares a line (the title and the y offset, the x
    /// axis name and the x offset) gets a line of its own when the two would collide.
    fn layout(&self, graph: &Graph, view: &Limits<f64>) -> Result<Layout> {
        let (layout, collisions) = self.layout_with(graph, view, Stacking::default())?;
        if collisions == Stacking::default() {
            return Ok(layout);
        }
        // stacking only adds lines, so this pass has nothing left to collide
        Ok(self.layout_with(graph, view, collisions)?.0)
    }

    /// One layout pass with `stacking` applied; also returns the shared lines that collide.
    fn layout_with(
        &self,
        graph: &Graph,
        view: &Limits<f64>,
        stacking: Stacking,
    ) -> Result<(Layout, Stacking)> {
        let (outer_min, outer_max) = self.buffered_area();

        let largest_marker_sz = graph
            .data()
            .iter()
            // thick lines extend past the data points just like markers do
            .map(|s| {
                let line_thickness = s.line_style().map_or(0, |l| l.thickness());
                s.marker_style().size().max(line_thickness)
            })
            .max()
            .ok_or(Error::NoData)?;

        // axes are drawn just outside the plot area, so leave room for their thickness
        let axes = graph.axes().cloned();
        let (axes_inset, show_x_labels, show_y_labels) =
            match axes.as_ref().map(|a| a.positioning()) {
                Some(AxesPositioning::XOnly(line)) => {
                    ((0, line.thickness().saturating_mul(2)), true, false)
                }
                Some(AxesPositioning::YOnly(line)) => {
                    ((line.thickness().saturating_mul(2), 0), false, true)
                }
                Some(AxesPositioning::XY(line)) => {
                    let inset = line.thickness().saturating_mul(2);
                    ((inset, inset), true, true)
                }
                None => ((0, 0), false, false),
            };
        // axes and markers can overlap, so use the larger of the two
        let inset_x = largest_marker_sz.max(axes_inset.0);
        let inset_y = largest_marker_sz.max(axes_inset.1);

        let (color, size) = self.label_style(axes.as_ref());
        let title_size = ((size as f32 * TITLE_SCALE).round() as u32).clamp(1, MAX_FONT_SIZE);
        let font = &self.font;
        let width = |text: &str, size: u32| font.width(text, size).ceil() as u32;
        let place = |text: &str, size: u32, left: u32, top: u32| {
            PlacedText::new(font, text, size, color, left, top)
        };
        let tick = font.metrics(size);
        let text_h = tick.height();
        let title_h = font.metrics(title_size).height();
        // gaps and line spacing scale with the text: em(0.35) is 0.35 of the tick label size
        let em = |fraction: f32| (fraction * size as f32).round() as u32;
        let (tick_gap, name_gap) = (em(TICK_GAP), em(NAME_GAP));
        let (title_gap, offset_gap) = (em(TITLE_GAP), em(OFFSET_GAP));
        let pitch = |size: u32| (size as f32 * LINE_SPACING).round() as u32;
        // height of a block of `lines` wrapped lines, each `line_h` tall
        let block = |lines: usize, line_h: u32, size: u32| match lines {
            0 => 0,
            n => (n as u32 - 1) * pitch(size) + line_h,
        };

        // Far from zero, f64 can't hold evenly spaced ticks, and the labels get long. As in
        // matplotlib, such an axis gets an offset (`+1e15`) that its ticks are fitted, placed and
        // labeled relative to. It's decided first because its label changes the margins.
        let x_offset = axis_offset(view.min().x, view.max().x);
        let y_offset = axis_offset(view.min().y, view.max().y);
        let (x_min, x_max) = (view.min().x - x_offset, view.max().x - x_offset);
        let (y_min, y_max) = (view.min().y - y_offset, view.max().y - y_offset);
        let x_offset_text = format_offset(x_offset).filter(|_| show_x_labels);
        let y_offset_text = format_offset(y_offset).filter(|_| show_y_labels);

        // the title and x name wrap to the width inside the buffer; as with matplotlib's
        // `axis("off")`, an axis without tick labels has no name either
        let outer_w = (outer_max.x.saturating_sub(outer_min.x) + 1) as f32;
        let title_lines = graph.title().map_or_else(Vec::new, |t| {
            wrap(t, outer_w, TITLE_MAX_LINES, |s| font.width(s, title_size))
        });
        let x_name_lines = match graph.x_label() {
            Some(t) if show_x_labels => wrap(t, outer_w, NAME_MAX_LINES, |s| font.width(s, size)),
            _ => Vec::new(),
        };
        let title_block = block(title_lines.len(), title_h, title_size);
        let x_name_block = block(x_name_lines.len(), text_h, size);

        // bottom band, from the edge up: the x offset when it has a line of its own, the x
        // name, then the tick labels
        let x_offset_alone =
            x_offset_text.is_some() && (x_name_lines.is_empty() || stacking.x_offset);
        let x_offset_line = if x_offset_alone { text_h + offset_gap } else { 0 };
        let x_name_band = if x_name_lines.is_empty() { 0 } else { x_name_block + name_gap };
        let below_ticks = x_offset_line + x_name_band;
        let bottom = if show_x_labels { below_ticks + text_h + tick_gap } else { 0 };

        // top band, from the plot up: the y offset, and the title on the same line unless the
        // two are stacked; the title also clears the top y tick label, which reaches half a
        // line above the plot
        let title_lift = if y_offset_text.is_some() && stacking.title {
            offset_gap + text_h + title_gap
        } else {
            title_gap.max(text_h / 2 + 1)
        };
        let y_offset_band = if y_offset_text.is_some() { offset_gap + text_h } else { 0 };
        let top = if !title_lines.is_empty() {
            (title_lift + title_block).max(y_offset_band)
        } else if y_offset_text.is_some() {
            y_offset_band
        } else if show_y_labels {
            text_h / 2
        } else {
            0
        };

        // saturating: huge markers, lines, text or buffers must end in CanvasTooSmall, not
        // overflow
        let plot_min_y = outer_min.y.saturating_add(bottom).saturating_add(inset_y);
        let plot_max_y = outer_max.y.saturating_sub(top.saturating_add(inset_y));
        check_area(
            &Point::new(outer_min.x, plot_min_y),
            &Point::new(outer_max.x, plot_max_y),
        )?;

        let (canvas_max_x, canvas_max_y) = (self.limits.max().x, self.limits.max().y);
        // the top row of the x tick labels
        let x_labels_top = outer_min.y + below_ticks + text_h - 1;
        // y tick labels are centered on their tick by the digits (matplotlib's
        // `center_baseline`), stay inside the canvas, and stay a row above the x tick labels
        let digit_center = (tick.ascent - tick.digit_height / 2.0).round() as u32;
        let top_lo = if show_x_labels {
            x_labels_top + text_h + 1
        } else {
            text_h.saturating_sub(1)
        };
        let top_hi = canvas_max_y.max(top_lo);
        let y_label_top = |value: f64| {
            let y = to_canvas(value, y_min, y_max, plot_min_y as f64, plot_max_y as f64);
            (y.round() as u32)
                .saturating_add(digit_center)
                .clamp(top_lo, top_hi)
        };

        // y ticks depend only on the plot height; their labels then set the left margin
        let y_ticks = fit_ticks(
            y_min,
            y_max,
            (plot_max_y - plot_min_y) as f64,
            |ticks, spacing| {
                // the clamp above can push end labels into their neighbours
                let tops: Vec<u32> = ticks.values.iter().map(|&v| y_label_top(v)).collect();
                spacing >= text_h as f64 + Y_LABEL_SPACING
                    && tops.windows(2).all(|t| t[1] >= t[0] + text_h)
            },
        );

        // left band: the y name, wrapped to the plot height and turned to read upwards, then
        // the y tick labels
        let plot_h = (plot_max_y - plot_min_y + 1) as f32;
        let y_name_lines = match graph.y_label() {
            Some(t) if show_y_labels => wrap(t, plot_h, NAME_MAX_LINES, |s| font.width(s, size)),
            _ => Vec::new(),
        };
        let y_name_band = if y_name_lines.is_empty() {
            0
        } else {
            block(y_name_lines.len(), text_h, size) + name_gap
        };
        let y_label_w = if show_y_labels {
            (y_ticks.labels.iter())
                .map(|l| width(l, size))
                .max()
                .unwrap_or(0)
        } else {
            0
        };
        let left = if show_y_labels {
            y_name_band + y_label_w + tick_gap
        } else {
            0
        };
        let plot_min_x = outer_min.x.saturating_add(left).saturating_add(inset_x);
        let plot_max_x = outer_max.x.saturating_sub(inset_x);
        let plot_min = Point::new(plot_min_x, plot_min_y);
        let plot_max = Point::new(plot_max_x, plot_max_y);
        check_area(&plot_min, &plot_max)?;

        let x_label_center = |value: f64, w: u32| {
            let x = to_canvas(value, x_min, x_max, plot_min_x as f64, plot_max_x as f64);
            // keep labels at the ends of the axis inside the canvas
            let lo = w / 2;
            let hi = canvas_max_x.saturating_sub(w - w / 2).max(lo);
            (x.round() as u32).clamp(lo, hi)
        };
        let x_ticks = fit_ticks(
            x_min,
            x_max,
            (plot_max_x - plot_min_x) as f64,
            |ticks, spacing| {
                let widths: Vec<u32> = ticks.labels.iter().map(|l| width(l, size)).collect();
                let widest = widths.iter().copied().max().unwrap_or(0);
                // the clamp in x_label_center can push end labels into their neighbours
                let spans: Vec<(u32, u32)> = (ticks.values.iter().zip(&widths))
                    .map(|(&v, &w)| {
                        let left = x_label_center(v, w) - w / 2;
                        (left, left + w)
                    })
                    .collect();
                spacing >= widest as f64 + X_LABEL_SPACING
                    && spans
                        .windows(2)
                        .all(|s| s[1].0 as f64 >= s[0].1 as f64 + X_LABEL_SPACING / 2.0)
            },
        );

        let plot = Limits::new(plot_min, plot_max);
        let mut tick_labels = Vec::new();
        if show_x_labels {
            for (&value, label) in x_ticks.values.iter().zip(&x_ticks.labels) {
                let w = width(label, size);
                tick_labels.push(place(label, size, x_label_center(value, w) - w / 2, x_labels_top));
            }
        }
        if show_y_labels {
            // right-aligned against the plot area
            let right = outer_min.x + y_name_band + y_label_w;
            for (&value, label) in y_ticks.values.iter().zip(&y_ticks.labels) {
                let left = right - width(label, size);
                tick_labels.push(place(label, size, left, y_label_top(value)));
            }
        }

        // the title and x name are centered on the plot, kept inside the buffered area
        let plot_center_x = (plot_min_x + plot_max_x) / 2;
        let centered = |w: u32| {
            (plot_center_x.saturating_sub(w / 2))
                .min((outer_max.x + 1).saturating_sub(w))
                .max(outer_min.x)
        };

        // the x name below the tick labels, first line on top
        let x_name_top = x_labels_top.saturating_sub(text_h + name_gap);
        let x_name: Vec<PlacedText> = (x_name_lines.iter().enumerate())
            .map(|(i, line)| {
                let top = x_name_top.saturating_sub(i as u32 * pitch(size));
                place(line, size, centered(width(line, size)), top)
            })
            .collect();
        // where matplotlib puts it: under the right end of the x axis, on the x name's first
        // line or on a line of its own
        let x_offset_label = x_offset_text.map(|text| {
            let top = if x_offset_alone {
                outer_min.y + text_h - 1
            } else {
                x_name_top
            };
            place(&text, size, (plot_max_x + 1).saturating_sub(width(&text, size)), top)
        });

        // above the plot: the y offset at the y axis, and the title, first line on top
        let band_start = plot_max_y + inset_y + 1;
        let y_offset_label = y_offset_text.map(|text| {
            let left = plot_min_x.saturating_sub(axes_inset.0);
            place(&text, size, left, band_start + offset_gap + text_h - 1)
        });
        let title_top = band_start + title_lift + title_block.saturating_sub(1);
        let title: Vec<PlacedText> = (title_lines.iter().enumerate())
            .map(|(i, line)| {
                let top = title_top.saturating_sub(i as u32 * pitch(title_size));
                place(line, title_size, centered(width(line, title_size)), top)
            })
            .collect();

        // the y name: each line turned to read upwards, the first farthest from the plot, all
        // centered on the plot vertically
        let plot_center_y = (plot_min_y + plot_max_y) / 2;
        let y_name: Vec<PlacedText> = (y_name_lines.iter().enumerate())
            .map(|(i, line)| {
                let mut text = place(line, size, outer_min.x + i as u32 * pitch(size), 0);
                text.coverage = text.coverage.rotated_ccw();
                text.top = plot_center_y + text.coverage.height / 2;
                text
            })
            .collect();

        // shared lines whose text would come closer than MIN_APART
        let apart = em(MIN_APART);
        let collide = |a: Option<&PlacedText>, b: Option<&PlacedText>| {
            match (a.and_then(PlacedText::bounds), b.and_then(PlacedText::bounds)) {
                (Some(a), Some(b)) => horizontal_gap(&a, &b) < apart,
                _ => false,
            }
        };
        let collisions = Stacking {
            title: !stacking.title && collide(title.last(), y_offset_label.as_ref()),
            x_offset: !x_offset_alone && collide(x_name.first(), x_offset_label.as_ref()),
        };

        Ok((
            Layout {
                plot,
                x_ticks,
                y_ticks,
                x_offset,
                y_offset,
                tick_labels,
                x_offset_label,
                y_offset_label,
                title,
                x_name,
                y_name,
            },
            collisions,
        ))
    }
```

- [ ] **Step 6: Run the tests to confirm they pass**

Run: `cargo test --lib`

Expected: everything passes, including the existing offset, overlap and font-size tests.

- [ ] **Step 7: Regenerate and review the images, run the gate, commit**

Run: `TERMPLT_UPDATE_SNAPSHOTS=1 cargo test --test golden && scripts/readme_images.sh`

Look at every changed PNG. The gaps are now about 5 px at 14 px text, y labels sit centered on their grid lines, and the offset labels are where the tests put them. Then run the full gate.

```bash
git add src/plotting tests/snapshots docs/images
git commit -F - <<'EOF'
Lay out a title and axis names in bands scaled to the text

Gaps follow matplotlib's paddings as fractions of the text size, y tick labels are centered
on their digits, titles and axis names wrap (the y name reads upwards), and a title or x
offset that would collide with the text on its shared line moves to a line of its own.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 9: `Plot` API and terminal-matched text size

**Files:**
- Modify: `src/plot.rs`, `src/terminal.rs`

**Interfaces:**
- Consumes: `Graph::with_title`, `with_x_label`, `with_y_label`; `TerminalCanvas::with_font`, `with_font_size`; `Font`; `DEFAULT_FONT_SIZE`; `MAX_FONT_SIZE`.
- Produces:
  - `Plot::title`, `x_label`, `y_label` (each `impl Into<String>`), `Plot::font(Font)` and `Plot::font_size(u32)`.
  - `Terminal::text_size(&self) -> u32`.
  - Test-only: `Terminal::with_window(WindowSize) -> Terminal`.
  - Private: `Plot::font_size_in(&self, &Terminal) -> u32`.

- [ ] **Step 1: Write the failing tests**

In the tests module of `src/terminal.rs`:

```rust
    #[test]
    fn text_size_matches_the_terminal_rows() {
        let size = |pix_per_row| {
            text_size_for(&WindowSize {
                rows: 24,
                cols: 80,
                x_pix: 800,
                y_pix: 24 * pix_per_row,
                pix_per_row,
                pix_per_col: 10,
            })
        };
        assert_eq!(size(34), 28); // a 2x (Retina) terminal with 17 pt rows
        assert_eq!(size(17), 14);
        assert_eq!(size(9), 8); // never below 8 px
        assert_eq!(size(0), 8); // a terminal that reports no row height
        assert_eq!(size(1000), MAX_FONT_SIZE);
    }
```

In the tests module of `src/plot.rs`:

```rust
    #[test]
    fn titles_and_names_reach_the_graph() {
        let graph = Plot::new()
            .line(vec![(0, 0), (1, 1)])
            .title("T")
            .x_label("x")
            .y_label("y")
            .graph();
        assert_eq!(graph.title(), Some("T"));
        assert_eq!(graph.x_label(), Some("x"));
        assert_eq!(graph.y_label(), Some("y"));
    }

    #[test]
    fn font_size_sets_the_text_size_of_the_canvas() {
        let plot = Plot::new().line(vec![(0, 0), (1, 1)]);
        let area = |p: &Plot| p.canvas(800, 600).get_drawable_limits().unwrap().span();
        assert_eq!(area(&plot), area(&plot.clone().font_size(DEFAULT_FONT_SIZE)));
        // bigger text leaves less room for the plot
        assert!(area(&plot.clone().font_size(40)).1 < area(&plot).1);
    }

    #[test]
    fn show_matches_the_terminal_text_unless_a_size_is_set() {
        let terminal = Terminal::with_window(crate::terminal::WindowSize {
            rows: 50,
            cols: 160,
            x_pix: 1600,
            y_pix: 1700,
            pix_per_row: 34,
            pix_per_col: 10,
        });
        assert_eq!(Plot::new().font_size_in(&terminal), 28);
        assert_eq!(Plot::new().font_size(20).font_size_in(&terminal), 20);
    }
```

- [ ] **Step 2: Run them to confirm they fail**

Run: `cargo test --lib -- plot::tests terminal::tests`

Expected: the build fails because `text_size_for`, `Plot::title`, `font_size`, `font_size_in` and `Terminal::with_window` don't exist.

- [ ] **Step 3: Implement `Terminal::text_size`**

In `src/terminal.rs`, add `use crate::plotting::text::MAX_FONT_SIZE;`, then add inside `impl Terminal` after `default_plot_size`:

```rust
    /// The base text size in pixels that matches the terminal's own text: a terminal row is
    /// about 1.2 em, so the row height divided by 1.2, at least 8 and at most
    /// [`MAX_FONT_SIZE`].
    pub fn text_size(&self) -> u32 {
        text_size_for(&self.window)
    }
```

After `estimate_window_size`:

```rust
/// See [`Terminal::text_size`].
fn text_size_for(window: &WindowSize) -> u32 {
    ((window.pix_per_row as f32 / 1.2).round() as u32).clamp(8, MAX_FONT_SIZE)
}

#[cfg(test)]
impl Terminal {
    /// A terminal of a given size, for tests elsewhere in the crate.
    pub(crate) fn with_window(window: WindowSize) -> Terminal {
        Terminal {
            window,
            passthrough: Passthrough::None,
        }
    }
}
```

- [ ] **Step 4: Implement the `Plot` additions**

In `src/plot.rs`:
- Add `font::Font` and `text::{DEFAULT_FONT_SIZE, TextStyle}` to the `plotting::{...}` import.
- Add to `struct Plot`:

```rust
    title: Option<String>,
    x_label: Option<String>,
    y_label: Option<String>,
    font: Font,
    font_size: Option<u32>,
```

- Add `title: None, x_label: None, y_label: None, font: Font::default(), font_size: None,` to `Default`.
- Add these methods after `grid`:

```rust
    /// Sets the title, drawn above the plot. Long titles wrap onto up to 3 lines.
    pub fn title(mut self, text: impl Into<String>) -> Self {
        self.title = Some(text.into());
        self
    }

    /// Names the x axis.
    pub fn x_label(mut self, text: impl Into<String>) -> Self {
        self.x_label = Some(text.into());
        self
    }

    /// Names the y axis (drawn turned to read upwards).
    pub fn y_label(mut self, text: impl Into<String>) -> Self {
        self.y_label = Some(text.into());
        self
    }

    /// Sets the font for all text; characters it lacks are drawn with the built-in Go font.
    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Sets the base text size in pixels: tick labels and axis names use it, and the title is
    /// 1.2 times larger. By default [`Plot::show`] matches the terminal's text
    /// ([`Terminal::text_size`]), and other output uses [`DEFAULT_FONT_SIZE`].
    pub fn font_size(mut self, px: u32) -> Self {
        self.font_size = Some(px);
        self
    }
```

- In `graph()`, before `graph` is returned:

```rust
        if let Some(text) = &self.title {
            graph = graph.with_title(text.clone());
        }
        if let Some(text) = &self.x_label {
            graph = graph.with_x_label(text.clone());
        }
        if let Some(text) = &self.y_label {
            graph = graph.with_y_label(text.clone());
        }
```

- Replace `canvas` and `show_in` with:

```rust
    /// The canvas the plot is drawn on, for callers that want to adjust it before drawing.
    pub fn canvas(&self, width: u32, height: u32) -> TerminalCanvas {
        self.canvas_with_font_size(width, height, self.font_size.unwrap_or(DEFAULT_FONT_SIZE))
    }

    fn canvas_with_font_size(&self, width: u32, height: u32, font_size: u32) -> TerminalCanvas {
        // tick labels are laid out inside the canvas automatically; the buffer is just
        // breathing room around the edges
        let buffer = (width.min(height) / 40).max(8);
        TerminalCanvas::new(width, height, self.background)
            .with_buffer(BufferType::Uniform(buffer))
            .with_font(self.font.clone())
            .with_font_size(font_size)
            .with_graph(self.graph())
    }

    /// The base text size [`Plot::show_in`] uses: the one set, or the terminal's.
    fn font_size_in(&self, terminal: &Terminal) -> u32 {
        self.font_size.unwrap_or_else(|| terminal.text_size())
    }
```

```rust
    /// Like [`Plot::show`], with a [`Terminal`] that was already connected.
    pub fn show_in(&self, terminal: &Terminal) -> Result<()> {
        let (width, height) = self.size.unwrap_or_else(|| terminal.default_plot_size());
        let rgb = self
            .canvas_with_font_size(width, height, self.font_size_in(terminal))
            .draw()?
            .into_bytes();
        terminal.show_rgb(&rgb, width, height)
    }
```

- [ ] **Step 5: Run the tests, then the full gate, and commit**

Run: `cargo test --lib -- plot::tests terminal::tests`

Expected: all pass. Then run the full gate.

```bash
git add src/plot.rs src/terminal.rs
git commit -F - <<'EOF'
Add titles, axis names, fonts and text sizes to Plot

Plot::show matches the terminal's own text size (Terminal::text_size: the row height / 1.2);
PNGs and render() use 14 px unless font_size is set.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 10: Golden scenes and property tests for text, then open PR 2

**Files:**
- Modify: `tests/golden.rs`, `tests/properties.rs`, `src/plotting/canvas.rs` (a property test), `CHANGELOG.md`
- Create: `tests/snapshots/{title_and_axis_names,long_text_wraps,light_background_text}.png`

**Interfaces:** consumes the public API from Tasks 8 and 9.

- [ ] **Step 1: Add the scenes** (append to `tests/golden.rs`)

```rust
#[test]
fn title_and_axis_names() {
    // both axes have offsets too, so every kind of text is on the plot
    let (w, h) = (400, 300);
    let points: Vec<_> = (0..=20)
        .map(|i| Point::new(1_700_000_000.0 + 5.0 * i as f64, 1e15 + 0.125 * (i % 7) as f64))
        .collect();
    let series = Series::new(&points)
        .with_marker_style(MarkerStyle::None)
        .with_line_style(LineStyle::Solid {
            color: colors::GOLD,
            thickness: 0,
        });
    let canvas = TerminalCanvas::new(w, h, colors::BLACK)
        .with_buffer(BufferType::Uniform(8))
        .with_graph(
            Graph::new()
                .with_series(series)
                .with_axes(axes())
                .with_grid_lines(grid())
                .with_title("Sensor drift, Δt = 5 s")
                .with_x_label("Time (s)")
                .with_y_label("Reading (µV)"),
        );
    check("title_and_axis_names", w, h, canvas);
}

#[test]
fn long_text_wraps() {
    let (w, h) = (320, 240);
    let series = Series::new(&curve(11, 0.0, 1.0, |x| x * x)).with_line_style(LineStyle::Solid {
        color: colors::CYAN,
        thickness: 0,
    });
    let canvas = TerminalCanvas::new(w, h, colors::BLACK)
        .with_buffer(BufferType::Uniform(8))
        .with_graph(
            Graph::new()
                .with_series(series)
                .with_axes(axes())
                .with_grid_lines(grid())
                .with_title(
                    "A title far too long to fit on one line of this small canvas, so it \
                     wraps and is finally cut off with an ellipsis after three lines of text",
                )
                .with_x_label("an axis name that is also much too long for this canvas")
                .with_y_label("a long y axis name, wrapped to the plot height"),
        );
    check("long_text_wraps", w, h, canvas);
}

#[test]
fn light_background_text() {
    // dark anti-aliased text on white: blending has to darken towards black correctly
    let (w, h) = (320, 240);
    let series = Series::new(&curve(9, -2.0, 0.5, |x| x * x * x)).with_line_style(
        LineStyle::Solid {
            color: colors::RED,
            thickness: 1,
        },
    );
    let canvas = TerminalCanvas::new(w, h, colors::WHITE)
        .with_buffer(BufferType::Uniform(8))
        .with_graph(
            Graph::new()
                .with_series(series)
                .with_axes(Axes::new(
                    AxesPositioning::XY(LineStyle::Solid {
                        color: colors::BLACK,
                        thickness: 1,
                    }),
                    TextStyle::with_color(colors::BLACK),
                ))
                .with_grid_lines(GridLines::XY(LineStyle::Solid {
                    color: colors::LIGHT_GRAY,
                    thickness: 0,
                }))
                .with_title("Dark text on white")
                .with_x_label("x")
                .with_y_label("x³"),
        );
    check("light_background_text", w, h, canvas);
}
```

- [ ] **Step 2: Confirm they fail, then create and review the snapshots**

Run: `cargo test --test golden`

Expected: 3 failures of the form "missing snapshot …".

Run: `TERMPLT_UPDATE_SNAPSHOTS=1 cargo test --test golden`, then open the three new PNGs:
- `title_and_axis_names` shows the title, both names and both offsets, with nothing overlapping.
- `long_text_wraps` has a 3-line title ending in `…` and 2-line names.
- `light_background_text` has crisp dark text.

Then run `cargo test --test golden` again.

- [ ] **Step 3: Add the property tests**

In `tests/properties.rs`, add this strategy near the others:

```rust
/// Titles and axis names: absent, blank, any printable text, or far too long.
fn text() -> impl Strategy<Value = Option<String>> {
    prop::option::of(prop_oneof![
        4 => "\\PC{0,40}",
        1 => "[a-z ]{150,400}",
        1 => Just("\n \n".to_string()),
    ])
}
```

Add `title in text(), x_label in text(), y_label in text(),` to the parameters of `draw_never_panics`, and before the `// Ok or Err are both acceptable` comment add:

```rust
        if let Some(text) = title {
            graph = graph.with_title(text);
        }
        if let Some(text) = x_label {
            graph = graph.with_x_label(text);
        }
        if let Some(text) = y_label {
            graph = graph.with_y_label(text);
        }
```

In the tests module of `src/plotting/canvas.rs`:

```rust
    proptest::proptest! {
        #[test]
        fn titles_and_names_stay_inside_the_canvas(
            title in "\\PC{0,80}",
            x_label in "\\PC{0,80}",
            y_label in "\\PC{0,80}",
            w in 150u32..900,
            h in 120u32..700,
            font_size in 6u32..40,
        ) {
            let graph = plain_graph()
                .with_title(title)
                .with_x_label(x_label)
                .with_y_label(y_label);
            let canvas = TerminalCanvas::new(w, h, colors::BLACK)
                .with_buffer(BufferType::Uniform(8))
                .with_font_size(font_size)
                .with_graph(graph.clone());
            // a canvas too small for the text is an error, which is fine
            if let Ok(layout) = canvas.layout(&graph, &graph.view_limits().unwrap()) {
                for text in layout.title.iter().chain(&layout.x_name).chain(&layout.y_name) {
                    if let Some(b) = text.bounds() {
                        proptest::prop_assert!(b.max().x < w && b.max().y < h, "{b:?} outside {w}x{h}");
                    }
                }
            }
        }
    }
```

Run: `cargo test --test properties && cargo test --lib titles_and_names_stay_inside_the_canvas`. Expected: pass.

- [ ] **Step 4: Changelog**

Add under `### Added` in the 0.3.0 section of `CHANGELOG.md`:

```markdown
- Titles and axis names: `Plot::title`, `x_label` and `y_label`, and `Graph::with_title`,
  `with_x_label` and `with_y_label`. Long text wraps (titles onto 3 lines, names onto 2) and
  ends with `…` when it still doesn't fit; the y name reads upwards.
- `Plot::font` and `Plot::font_size`. `Plot::show` matches the terminal's text size
  (`Terminal::text_size`); PNGs use 14 px.
```

and under `### Changed`:

```markdown
- Gaps around tick labels, names and the title scale with the text size (matplotlib's
  paddings), and y tick labels are centered on their digits.
```

- [ ] **Step 5: Run the full gate, commit, and open PR 2**

```bash
git add tests src/plotting/canvas.rs CHANGELOG.md
git commit -F - <<'EOF'
Add golden scenes and property tests for titles and axis names

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
git push -u origin titles-and-axis-names
gh pr create --base main --title "Titles and axis names (0.3.0, part 2)" --body "$(cat <<'EOF'
## Summary
Part 2 of spec 1 (`docs/superpowers/specs/2026-09-25-text-rendering-design.md`).

- `Plot::title`, `x_label`, `y_label`, `font`, `font_size`; `Graph::with_title`, `with_x_label`, `with_y_label`
- layout bands with matplotlib's paddings as fractions of the text size; y tick labels centered on their digits
- titles wrap onto 3 lines and names onto 2, ending with `…` when they still don't fit; the y name reads upwards
- a title or x offset that would collide with its shared line moves to its own line (two-pass layout)
- `Plot::show` matches the terminal's text size (`Terminal::text_size`); PNGs use 14 px

## Test plan
- [x] unit tests for wrapping, rotation, gaps, digit centering, collisions, no overlaps at 3 sizes
- [x] golden scenes `title_and_axis_names`, `long_text_wraps`, `light_background_text` (reviewed)
- [x] property tests: random titles and names never panic and stay inside the canvas
- [x] fmt, clippy, tests (with and without default features), docs
- [ ] CI on Linux, macOS, Windows and MSRV 1.88

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

(Use `--base text-engine` instead if PR 1 wasn't merged when this branch was made.) Watch CI with `gh pr checks --watch`.

---

# PR 3: CLI and docs

```bash
# once PR 2 is merged; otherwise branch from titles-and-axis-names and use it as the PR base
git checkout main && git pull --ff-only && git checkout -b cli-text
```

### Task 11: Name the axes from CSV headers

**Files:**
- Modify: `src/bin/termplt/data.rs`, `src/bin/termplt/main.rs`
- Regenerate: `docs/images/*.png`

**Interfaces:**
- Consumes: `Plot::x_label`, `Plot::y_label`.
- Produces:
  - `pub struct ColumnNames { pub x: Option<String>, pub y: Option<String> }` (Debug, Clone, Default, PartialEq, Eq).
  - `Parsed.names: ColumnNames`.
  - `pub fn axis_names(series: &[ColumnNames]) -> ColumnNames`.
  - `load_points` now returns `Result<(Vec<Point<f64>>, ColumnNames)>`.

- [ ] **Step 1: Write the failing tests** (add to the tests module in `data.rs`)

```rust
    fn names(x: Option<&str>, y: Option<&str>) -> ColumnNames {
        ColumnNames {
            x: x.map(str::to_string),
            y: y.map(str::to_string),
        }
    }

    #[test]
    fn points_report_the_header_names_of_their_columns() {
        let table = Table::parse("time,temp,humidity\n0,20,50\n1,21,51\n");
        let named = |x: Option<Column>, y: Option<Column>| {
            table.points(x.as_ref(), y.as_ref(), "t").unwrap().names
        };
        assert_eq!(named(None, None), names(Some("time"), Some("temp")));
        // -x 3
        assert_eq!(named(Some(Column::Index(2)), None), names(Some("humidity"), Some("temp")));
        // names match ignoring case, but the header's own text is reported
        assert_eq!(
            named(None, Some(Column::Name("HUMIDITY".into()))),
            names(Some("time"), Some("humidity"))
        );
        // -x index
        assert_eq!(named(Some(Column::RowNumber), None), names(None, Some("temp")));
    }

    #[test]
    fn columns_without_a_header_name_have_none() {
        let named = |content: &str| Table::parse(content).points(None, None, "t").unwrap().names;
        assert_eq!(named("0,20\n1,21\n"), ColumnNames::default());
        // a single column is plotted against the row number
        assert_eq!(named("temp\n20\n21\n"), names(None, Some("temp")));
        assert_eq!(named("time,\n0,20\n"), names(Some("time"), None));
        // quoted and padded header cells give clean names
        assert_eq!(named("\"time (s)\" , temp\n0,20\n"), names(Some("time (s)"), Some("temp")));
    }

    #[test]
    fn axes_are_named_only_when_every_series_agrees() {
        let temps = names(Some("time"), Some("temp"));
        let cases = [
            (vec![temps.clone()], names(Some("time"), Some("temp"))),
            // -y temp,humidity: two different y names
            (
                vec![temps.clone(), names(Some("time"), Some("humidity"))],
                names(Some("time"), None),
            ),
            // two files with the same header
            (vec![temps.clone(), temps.clone()], names(Some("time"), Some("temp"))),
            (vec![temps.clone(), names(Some("t"), Some("temp"))], names(None, Some("temp"))),
            // inline data has no names
            (vec![temps.clone(), ColumnNames::default()], names(None, None)),
            // compared exactly
            (vec![names(Some("Time"), Some("temp")), temps.clone()], names(None, Some("temp"))),
            (vec![], names(None, None)),
        ];
        for (series, expected) in cases {
            assert_eq!(axis_names(&series), expected, "{series:?}");
        }
    }
```

- [ ] **Step 2: Run them to confirm they fail**

Run: `cargo test --bin termplt data::tests`

Expected: the build fails with "cannot find type `ColumnNames`" and "no field `names`".

- [ ] **Step 3: Implement** (in `data.rs`)

Add `pub names: ColumnNames,` to `Parsed`. Then add this after `Parsed`:

```rust
/// The header names of the columns a series was read from; `None` where a column has none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColumnNames {
    pub x: Option<String>,
    pub y: Option<String>,
}

/// The names to put on the axes: an axis is named only when every series has a name for it and
/// all the names are the same (compared exactly).
pub fn axis_names(series: &[ColumnNames]) -> ColumnNames {
    fn agreed<'a>(mut names: impl Iterator<Item = &'a Option<String>>) -> Option<String> {
        let first = names.next()?.clone()?;
        names
            .all(|name| name.as_deref() == Some(first.as_str()))
            .then_some(first)
    }
    ColumnNames {
        x: agreed(series.iter().map(|s| &s.x)),
        y: agreed(series.iter().map(|s| &s.y)),
    }
}
```

In `impl Table`, add:

```rust
    /// The header text of a resolved column; `None` for the row number, a table without a
    /// header, or an empty header cell.
    fn header_name(&self, column: &Column) -> Option<String> {
        let Column::Index(i) = column else {
            return None;
        };
        let name = self.header.as_ref()?.get(*i)?.trim();
        (!name.is_empty()).then(|| name.to_string())
    }
```

In `Table::points`, right after `let mut parsed = Parsed::default();`, add:

```rust
        parsed.names = ColumnNames {
            x: self.header_name(&x),
            y: self.header_name(&y),
        };
```

In `main.rs`:
- Import `ColumnNames` (`use data::{Column, ColumnNames, Table};`).
- Change `load_points` to return `Result<(Vec<termplt::plotting::point::Point<f64>>, ColumnNames)>`:
  - its `match` yields `(points, 0, ColumnNames::default())` for inline data and `(parsed.points, parsed.skipped_missing, parsed.names)` for files;
  - destructure it as `let (points, skipped_missing, names) = ...`;
  - end with `Ok((points, names))`.
- In `run`, add `let mut column_names = Vec::new();` before the series loop, change the loop's first line to `let (points, names) = load_points(spec, &mut stdin_cache)?;`, and push `column_names.push(names);` at the end of the loop body. After the loop add:

```rust
    let names = data::axis_names(&column_names);
    if let Some(name) = names.x {
        plot = plot.x_label(name);
    }
    if let Some(name) = names.y {
        plot = plot.y_label(name);
    }
```

In the three `load_points` tests, take `.0` of the result: `load_points(&spec, &mut None).unwrap().0` and `load_points(&spec, &mut cache).unwrap().0.len()`.

- [ ] **Step 4: Run the tests, regenerate and review the README images**

Run: `cargo test --bin termplt && scripts/readme_images.sh`

Expected: tests pass. The sine, cosine and Lissajous images now show "x" under the x axis and "y" beside the y axis, from their `x,y` headers. Open them to check.

- [ ] **Step 5: Run the full gate and commit**

```bash
git add src/bin/termplt docs/images
git commit -F - <<'EOF'
Name the axes from CSV headers when every series agrees

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 12: `--title`, `--xlabel`, `--ylabel`, `--font` and `--font-size`

**Files:**
- Modify: `src/bin/termplt/cli.rs`, `src/bin/termplt/main.rs`, `tests/cli.rs`

**Interfaces:**
- Consumes:
  - `data::axis_names` and `ColumnNames` from Task 11;
  - `Plot::{title, x_label, y_label, font, font_size}`;
  - `Font::from_bytes`, `Terminal::text_size`, `DEFAULT_FONT_SIZE`, `MAX_FONT_SIZE`.
- Produces: the `Cli` fields `title`, `xlabel`, `ylabel`, `font: Option<PathBuf>` and `font_size: Option<u32>`.

- [ ] **Step 1: Write the failing tests** (append to `tests/cli.rs`)

```rust
const HEADED: &str = "time,temp\n0,20\n1,21\n2,23\n";
const BARE: &str = "0,20\n1,21\n2,23\n";

/// Plots `csv` with `args` into a 320x240 PNG and returns its pixels.
fn render(csv: &str, args: &[&str]) -> image::RgbImage {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data.csv");
    std::fs::write(&data, csv).unwrap();
    let png = dir.path().join("plot.png");
    let mut all = vec![
        data.to_str().unwrap(),
        "-o",
        png.to_str().unwrap(),
        "--width",
        "320",
        "--height",
        "240",
    ];
    all.extend_from_slice(args);
    let out = termplt(&all, "");
    assert!(out.status.success(), "{}", stderr(&out));
    image::open(&png).unwrap().to_rgb8()
}

#[test]
fn a_title_is_drawn() {
    assert_ne!(render(BARE, &[]), render(BARE, &["--title", "Temperatures"]));
}

#[test]
fn a_title_may_start_with_a_hyphen() {
    assert_ne!(render(BARE, &["--title", "-5 dB"]), render(BARE, &[]));
}

#[test]
fn axes_are_named_from_the_header_unless_told_otherwise() {
    let bare = render(BARE, &[]);
    // the header names the axes...
    assert_ne!(render(HEADED, &[]), bare);
    // ...an empty --xlabel/--ylabel removes those names...
    assert_eq!(render(HEADED, &["--xlabel", "", "--ylabel", ""]), bare);
    // ...and explicit names replace them
    let named = ["--xlabel", "t", "--ylabel", "T"];
    assert_eq!(render(HEADED, &named), render(BARE, &named));
}

#[test]
fn font_errors_name_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("plot.png");
    let run = |font: &Path| {
        termplt(
            &["-d", "(1,1),(2,2)", "-o", png.to_str().unwrap(), "--font", font.to_str().unwrap()],
            "",
        )
    };
    let out = run(&dir.path().join("missing.ttf"));
    assert!(!out.status.success());
    assert!(stderr(&out).contains("cannot read font"), "{}", stderr(&out));
    assert!(stderr(&out).contains("missing.ttf"), "{}", stderr(&out));

    let notes = dir.path().join("notes.txt");
    std::fs::write(&notes, "not a font").unwrap();
    let out = run(&notes);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("is not a TrueType or OpenType font"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn font_size_must_be_at_least_one() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("plot.png");
    let args = ["-d", "(1,1),(2,2)", "-o", png.to_str().unwrap(), "--font-size", "0"];
    let out = termplt(&args, "");
    assert!(!out.status.success());
    assert!(stderr(&out).contains("--font-size"), "{}", stderr(&out));
}
```

- [ ] **Step 2: Run them to confirm they fail**

Run: `cargo test --test cli`

Expected: the new tests fail with clap's "unexpected argument '--title'" (and the same for `--xlabel`, `--font`, `--font-size`). `axes_are_named_from_the_header_unless_told_otherwise` fails at its `--xlabel` render.

- [ ] **Step 3: Add the flags** (in `cli.rs`, after `output`)

```rust
    /// Plot title; long titles wrap onto up to 3 lines
    #[arg(long, value_name = "TEXT", allow_hyphen_values = true, help_heading = "Text")]
    pub title: Option<String>,

    /// X axis name [default: the x column's header, when every series has the same one]; ""
    /// for none
    #[arg(
        long,
        value_name = "TEXT",
        allow_hyphen_values = true,
        alias = "x-label",
        alias = "x_label",
        help_heading = "Text"
    )]
    pub xlabel: Option<String>,

    /// Y axis name [default: the y column's header, when every series has the same one]; ""
    /// for none
    #[arg(
        long,
        value_name = "TEXT",
        allow_hyphen_values = true,
        alias = "y-label",
        alias = "y_label",
        help_heading = "Text"
    )]
    pub ylabel: Option<String>,

    /// TrueType or OpenType font file for all text; characters it lacks use the built-in Go
    /// font
    #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath, help_heading = "Text")]
    pub font: Option<PathBuf>,

    /// Base text size in pixels; the title is 1.2 times larger [default: the terminal's text
    /// size, or 14 with --output]
    #[arg(
        long,
        value_name = "PX",
        value_parser = clap::value_parser!(u32).range(1..=i64::from(MAX_FONT_SIZE)),
        help_heading = "Text"
    )]
    pub font_size: Option<u32>,
```

Add `text::MAX_FONT_SIZE` to the `termplt::plotting::...` import in `cli.rs`, and add `  termplt data.csv --title "Temperatures" --ylabel "°C"` as a line in `EXAMPLES`.

- [ ] **Step 4: Wire them up** (in `main.rs`)

1. Imports: add `font::Font` and `text::DEFAULT_FONT_SIZE` to the `termplt::{...}` import.
2. Load the font right after `check_output_path`, so a bad font fails before any data is read:

```rust
    let font = cli.font.as_deref().map(load_font).transpose()?;
```

   and add the function after `check_output_path`:

```rust
/// Reads a font file for --font.
fn load_font(path: &Path) -> Result<Font> {
    let data =
        fs::read(path).map_err(|e| format!("cannot read font '{}': {e}", path.display()))?;
    Font::from_bytes(data).map_err(|_| {
        format!("'{}' is not a TrueType or OpenType font", path.display()).into()
    })
}
```

3. Replace the name wiring from Task 11 (the `let names = ...` block) with:

```rust
    // explicit names win; an empty one removes a name taken from the headers
    let names = data::axis_names(&column_names);
    if let Some(text) = cli.title.clone() {
        plot = plot.title(text);
    }
    if let Some(text) = cli.xlabel.clone().or(names.x) {
        plot = plot.x_label(text);
    }
    if let Some(text) = cli.ylabel.clone().or(names.y) {
        plot = plot.y_label(text);
    }
```

4. Replace `let plot = plot.size(width, height);` with:

```rust
    let (font_size, font_size_source) = match (cli.font_size, &terminal) {
        (Some(px), _) => (px, "--font-size".to_string()),
        (None, Some(terminal)) => (
            terminal.text_size(),
            format!("terminal rows are {} px", terminal.window().pix_per_row),
        ),
        (None, None) => (DEFAULT_FONT_SIZE, "default for --output".to_string()),
    };
    let mut plot = plot.size(width, height).font_size(font_size);
    if let Some(font) = font {
        plot = plot.font(font);
    }
    if cli.verbose {
        let font_name = cli
            .font
            .as_ref()
            .map_or_else(|| "Go (built in)".to_string(), |p| p.display().to_string());
        eprintln!("[verbose] text: {font_size} px ({font_size_source}), font: {font_name}");
    }
```

- [ ] **Step 5: Run the tests to confirm they pass**

Run: `cargo test --test cli && cargo test --bin termplt`

Expected: all pass. Also run `cargo run -q -- --help` and check that the Text heading lists the five flags.

- [ ] **Step 6: Run the full gate and commit**

```bash
git add src/bin/termplt tests/cli.rs
git commit -F - <<'EOF'
Add --title, --xlabel, --ylabel, --font and --font-size

Explicit names override the ones taken from CSV headers, and an empty name removes one.
In a terminal the text matches the terminal's own size; with --output it is 14 px.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 13: Docs, then open PR 3

**Files:**
- Modify: `README.md`, `scripts/readme_images.sh`, `CLAUDE.md`, `IMPROVEMENTS.md`, `CHANGELOG.md`
- Create: `docs/images/titles.png`

- [ ] **Step 1: README**

- **New CLI example:** after the "Line-only plot" example block and its image, add:

````markdown
```bash
# Title and axis names (by default the axes are named from the CSV header)
termplt test_data/sine.csv test_data/cosine.csv --title "Sine and cosine" --xlabel "angle (rad)" --ylabel "value"
```
<img width="600" height="450" alt="Sine and cosine curves with a title and named axes" src="https://raw.githubusercontent.com/EdCarney/termplt/main/docs/images/titles.png" />
````

- **Options table:** add after the `-o, --output` row:

```markdown
| `--title <TEXT>` | Plot title; long titles wrap onto up to 3 lines |
| `--xlabel <TEXT>` / `--ylabel <TEXT>` | Axis names (default: the column header when every series has the same one; `""` for none) |
| `--font <FILE>` | A `.ttf`/`.otf` font for all text; characters it lacks use the built-in Go font |
| `--font-size <PX>` | Base text size (default: matches the terminal's text, or 14 with `--output`) |
```

- **Data files paragraph:** append `Header names become axis names when every series agrees.` to the paragraph that starts "Data files may have a header row".
- **"Plotting in one call" example:** add `.title("Waves")` and `.x_label("x")` to the first `Plot::new()` chain, before `.show()?`.
- **Plot sentence:** change "Other options: ..." to list `.title(..)`, `.x_label(..)`, `.y_label(..)`, `.font(Font::from_bytes(..)?)`, `.font_size(px)` along with the existing ones.

In `scripts/readme_images.sh`, add before the final `echo`:

```bash
"$termplt" test_data/sine.csv test_data/cosine.csv --title "Sine and cosine" \
    --xlabel "angle (rad)" --ylabel "value" -o "$out/titles.png"
```

Run: `scripts/readme_images.sh`, then open `docs/images/titles.png` and check the title, names and curves.

- [ ] **Step 2: CLAUDE.md, IMPROVEMENTS.md, CHANGELOG.md**

- **CLAUDE.md:**
  - Pipeline diagram: replace the four `layout()` comment lines with:

```text
  ├── layout()                       # two passes: bands sized from measured text (title and y
  │                                  #   offset; x name and x offset; tick labels), gaps in em;
  │                                  #   stacked when shared lines would collide
```

  - Add to the `Plot` line in "Public API layers": `title`/`x_label`/`y_label`, `font`/`font_size` (terminal-matched in `show()`).
  - In the CLI section: `data.rs` also reports the header names of the resolved columns, and `axis_names` names an axis only when every series agrees. `main.rs` applies `--title`/`--xlabel`/`--ylabel` over those names, `--font`, and the text size (the terminal's, or 14 with `--output`).
  - Known Issues: replace the "No titles, axis names or legends yet" bullet with `- No legend yet (spec 2 of 0.3.0)`.
- **IMPROVEMENTS.md:**
  - Under "### 13.", add `◐ **Titles and axis names done (0.3.0):** TrueType text (embedded Go font, \`ab_glyph\`), \`Plot::title\`/\`x_label\`/\`y_label\`, \`--title\`/\`--xlabel\`/\`--ylabel\`, and names from CSV headers. The legend is next.`
  - In item 18, change "`--title` still needs a text font (item 13), and" to "`--title` is done (0.3.0);".
  - In item 32, change "`.title()` waits on item 13 (the font)." to "`.title()` is done (0.3.0)."
- **CHANGELOG.md**, under `### Added` in 0.3.0:

```markdown
- CLI: `--title`, `--xlabel`, `--ylabel`, `--font` and `--font-size`. Axes are named from CSV
  headers when every series has the same one (`--xlabel ""` removes a name). In a terminal the
  text matches the terminal's own size; with `--output` it is 14 px.
```

- [ ] **Step 3: Run the full gate, commit, and open PR 3**

```bash
git add README.md scripts/readme_images.sh docs/images CLAUDE.md IMPROVEMENTS.md CHANGELOG.md
git commit -F - <<'EOF'
Document titles, axis names and the text options

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
git push -u origin cli-text
gh pr create --base main --title "CLI titles, axis names and fonts (0.3.0, part 3)" --body "$(cat <<'EOF'
## Summary
Part 3 of spec 1 (`docs/superpowers/specs/2026-09-25-text-rendering-design.md`), which completes it.

- `--title`, `--xlabel`, `--ylabel`, `--font`, `--font-size` (new "Text" help heading)
- axes named from CSV headers when every series agrees; explicit flags win, `""` removes a name
- in a terminal the text matches the terminal's size; `--output` uses 14 px; `--verbose` says which
- README section and image, CLAUDE.md, IMPROVEMENTS.md and CHANGELOG updated

## Test plan
- [x] unit tests for header names and the agreement rule (including quoted headers)
- [x] CLI end to end: a title changes the PNG, header names and their overrides, hyphen-leading titles, font errors, `--font-size 0`
- [x] README images regenerated and reviewed
- [x] fmt, clippy, tests (with and without default features), docs
- [ ] CI on Linux, macOS, Windows and MSRV 1.88

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

(Use the stacked base branch if PR 2 wasn't merged.) Watch CI with `gh pr checks --watch`. 0.3.0 is not tagged here: spec 2 (the legend) comes first.
