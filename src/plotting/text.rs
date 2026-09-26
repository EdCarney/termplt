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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plotting::colors;

    #[test]
    fn text_style_sizes_are_clamped() {
        assert_eq!(TextStyle::new(colors::WHITE, 0).size(), Some(1));
        assert_eq!(TextStyle::new(colors::WHITE, 20).size(), Some(20));
        assert_eq!(
            TextStyle::new(colors::WHITE, 5000).size(),
            Some(MAX_FONT_SIZE)
        );
        assert_eq!(TextStyle::with_color(colors::WHITE).size(), None);
    }

    /// Every character is 10 px wide.
    fn fixed(s: &str) -> f32 {
        s.chars().count() as f32 * 10.0
    }

    #[test]
    fn short_text_stays_on_one_line() {
        assert_eq!(
            wrap("Oscillator response", 1000.0, 3, fixed),
            ["Oscillator response"]
        );
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
}
