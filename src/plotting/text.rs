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
}
