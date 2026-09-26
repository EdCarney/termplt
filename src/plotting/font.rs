//! TrueType fonts: the built-in Go font, fonts supplied by the user, and laying out a line of
//! text.

use crate::{Error, Result};
use ab_glyph::{Font as _, FontArc, GlyphId, PxScale, ScaleFont as _, point};
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
            glyphs.push(PositionedGlyph {
                font,
                id,
                x,
                advance,
            });
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
        assert!(
            missing.is_empty(),
            "missing from the built-in font: {missing}"
        );
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
            assert!(
                y > metrics.ascent - metrics.digit_height - 1.0,
                "row {y} is too high"
            );
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

    #[test]
    fn font_is_send_and_sync() {
        fn check<T: Send + Sync + 'static>() {}
        check::<Font>();
    }
}
