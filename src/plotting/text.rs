use super::{
    colors,
    common::{Drawable, IntConvertable, MaskPoints, UIntConvertable},
    limits::Limits,
    numbers,
    point::Point,
};
use crate::common::Result;
use rgb::RGB8;

/// Where a label is anchored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextPositioning {
    /// Centered on the point.
    Centered(Point<u32>),
    /// The left edge at the point, vertically centered.
    LeftAligned(Point<u32>),
    // to add...
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

#[derive(Debug, Clone)]
pub(crate) struct TextChar {
    bitmap: Vec<Vec<bool>>,
}

impl TextChar {
    pub(crate) fn new(value: char, style: &TextStyle) -> TextChar {
        let bitmap = numbers::get_bitmap(value, style);
        TextChar { bitmap }
    }

    pub(crate) fn width(&self) -> usize {
        self.bitmap.iter().map(|row| row.len()).max().unwrap_or(0)
    }

    pub(crate) fn height(&self) -> usize {
        self.bitmap.len()
    }

    pub(crate) fn get_mask(
        &self,
        lower_left: Point<u32>,
        style: TextStyle,
    ) -> Result<Vec<MaskPoints>> {
        let mut points = Vec::new();
        for i in 0..self.height() {
            for j in 0..self.width() {
                // pixels past u32::MAX can't be on any canvas
                if self.bitmap[i][j]
                    && let (Some(x), Some(y)) = (
                        lower_left.x.checked_add(j as u32),
                        lower_left.y.checked_add(i as u32),
                    )
                {
                    points.push(Point::new(x, y));
                }
            }
        }
        let color = style.color;
        Ok(vec![MaskPoints { points, color }])
    }
}

/// Color, scale and padding of bitmap text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextStyle {
    color: RGB8,
    scale: usize,
    padding: usize,
}

impl Default for TextStyle {
    fn default() -> TextStyle {
        TextStyle {
            color: colors::BLACK,
            scale: 1,
            padding: 1,
        }
    }
}

/// Largest glyph scale [`TextStyle::new`] accepts (320-pixel-wide digits).
pub const MAX_TEXT_SCALE: usize = 32;

/// Largest padding [`TextStyle::new`] accepts, in pixels.
pub const MAX_TEXT_PADDING: usize = 64;

impl TextStyle {
    /// Creates a text style. `scale` is the pixel multiplier for each glyph, clamped to
    /// `1..=`[`MAX_TEXT_SCALE`]; `padding` is the empty border around the text in pixels,
    /// clamped to [`MAX_TEXT_PADDING`]. The limits keep the glyph bitmaps small.
    pub fn new(color: RGB8, scale: usize, padding: usize) -> TextStyle {
        TextStyle {
            color,
            scale: scale.clamp(1, MAX_TEXT_SCALE),
            padding: padding.min(MAX_TEXT_PADDING),
        }
    }

    /// Text in `color` at scale 1 with a 1-pixel padding.
    pub fn with_color(color: RGB8) -> TextStyle {
        TextStyle {
            color,
            scale: 1,
            padding: 1,
        }
    }

    /// The text color.
    pub fn color(&self) -> RGB8 {
        self.color
    }

    /// The pixel scale factor (each font pixel becomes `scale` x `scale` pixels).
    pub fn scale(&self) -> usize {
        self.scale
    }

    /// Padding in pixels around each character.
    pub fn padding(&self) -> usize {
        self.padding
    }
}

/// A line of bitmap text. The font covers `0-9 . - + e` and space; other characters are drawn
/// as a box.
#[derive(Debug, Clone)]
pub struct Text {
    style: TextStyle,
    chars: Vec<TextChar>,
    width: usize,
    height: usize,
}

impl Text {
    /// Creates text in the given style.
    pub fn new(text: &str, style: TextStyle) -> Text {
        let chars = text
            .chars()
            .map(|c| TextChar::new(c, &style))
            .collect::<Vec<_>>();
        let width = chars.iter().fold(0usize, |acc, val| acc + val.width());
        let height = chars.iter().map(|c| c.height()).max().unwrap_or(0);

        Text {
            style,
            chars,
            width,
            height,
        }
    }

    /// Height in pixels.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Width in pixels.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Renders a number with `sig_figs` significant figures (at least 1).
    pub fn from_number(number: f64, sig_figs: usize, style: TextStyle) -> Text {
        Text::new(&num_to_str(number, sig_figs), style)
    }
}

/// Rounds to `sig_figs` significant figures. Plain decimals are used when the magnitude lies
/// between `0.1^sig_figs` and `10^sig_figs`, scientific notation otherwise; trailing zeros are
/// dropped.
fn num_to_str(number: f64, sig_figs: usize) -> String {
    let sig_figs = sig_figs.max(1);
    if number == 0.0 {
        return "0".to_string();
    }
    if !number.is_finite() {
        return number.to_string();
    }

    // formatting in scientific notation rounds to the right number of figures
    let scientific = format!("{:.*e}", sig_figs - 1, number);
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("{:e} output has an exponent");
    let exponent: i32 = exponent.parse().expect("{:e} exponents are integers");
    let rounded: f64 = scientific.parse().expect("{:e} output parses");

    let min = 0.1_f64.powi(sig_figs as i32);
    let max = 10_f64.powi(sig_figs as i32);
    if min < rounded.abs() && rounded.abs() < max {
        let decimals = (sig_figs as i32 - 1 - exponent).max(0) as usize;
        trim_trailing_zeros(format!("{rounded:.decimals$}"))
    } else {
        let mantissa = trim_trailing_zeros(mantissa.to_string());
        if exponent == 0 {
            mantissa
        } else {
            format!("{mantissa}e{exponent}")
        }
    }
}

fn trim_trailing_zeros(s: String) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// Text placed on the canvas.
#[derive(Debug, Clone)]
pub struct Label {
    txt: Text,
    pos: TextPositioning,
}

impl Label {
    /// Places `txt` at `pos`.
    pub fn new(txt: Text, pos: TextPositioning) -> Label {
        Label { txt, pos }
    }

    /// The text.
    pub fn txt(&self) -> &Text {
        &self.txt
    }

    /// Where it is placed.
    pub fn pos(&self) -> &TextPositioning {
        &self.pos
    }

    /// The pixels the label covers.
    pub fn limits(&self) -> Limits<u32> {
        match &self.pos {
            TextPositioning::Centered(center) => {
                let shift_floor = Point::new(
                    (self.txt.width as f64 / 2.).floor() as u32,
                    (self.txt.height as f64 / 2.).floor() as u32,
                );
                let shift_ceil = Point::new(
                    (self.txt.width as f64 / 2.).ceil() as u32,
                    (self.txt.height as f64 / 2.).ceil() as u32,
                );
                let min = Point::new(
                    center.x.saturating_sub(shift_floor.x),
                    center.y.saturating_sub(shift_floor.y),
                );
                // saturating: a label near u32::MAX is clipped rather than overflowing
                let max = Point::new(
                    center.x.saturating_add(shift_ceil.x),
                    center.y.saturating_add(shift_ceil.y),
                );
                Limits::new(min, max)
            }
            TextPositioning::LeftAligned(left) => {
                let shift_floor = (self.txt.height as f64 / 2.).floor() as u32;
                let shift_ceil = (self.txt.height as f64 / 2.).ceil() as u32;
                let min = Point::new(left.x, left.y.saturating_sub(shift_floor));
                let max = Point::new(
                    left.x.saturating_add(self.txt.width as u32),
                    left.y.saturating_add(shift_ceil),
                );
                Limits::new(min, max)
            }
        }
    }
}

impl Drawable for Label {
    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        // text sizes are bounded by the canvas, so these conversions saturate only in theory
        let height_shift = i32::try_from(self.txt.height / 2).unwrap_or(i32::MAX);
        // horizontal offset of the text's left edge from the anchor point
        let (anchor, mut x_offset) = match &self.pos {
            TextPositioning::Centered(center) => (
                center,
                -i32::try_from(self.txt.width / 2).unwrap_or(i32::MAX),
            ),
            TextPositioning::LeftAligned(left) => (left, 0),
        };

        let mut masks = Vec::new();
        for c in &self.txt.chars {
            let anchor = anchor.convert_to_i32();
            let char_lower_left = Point::new(
                anchor.x.saturating_add(x_offset),
                anchor.y.saturating_sub(height_shift),
            );
            masks.extend(c.get_mask(char_lower_left.convert_to_u32(), self.txt.style)?);
            x_offset = x_offset.saturating_add(i32::try_from(c.width()).unwrap_or(i32::MAX));
        }
        Ok(masks)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn label_limits_left_aligned_starts_at_anchor() {
        let txt = Text::new("0", TextStyle::default());
        let (width, height) = (txt.width() as u32, txt.height() as u32);
        let label = Label::new(txt, TextPositioning::LeftAligned(Point::new(50, 50)));
        let limits = label.limits();
        assert_eq!(limits.min().x, 50);
        assert_eq!(limits.max().x, 50 + width);
        assert_eq!(limits.min().y, 50 - height / 2);
    }

    #[test]
    fn label_get_mask_left_aligned_lies_right_of_anchor() {
        let txt = Text::new("10", TextStyle::default());
        let width = txt.width() as u32;
        let label = Label::new(txt, TextPositioning::LeftAligned(Point::new(50, 50)));
        let mask = label.get_mask().unwrap();
        let points: Vec<_> = mask.iter().flat_map(|m| m.points.iter()).collect();
        assert!(!points.is_empty());
        assert!(points.iter().all(|p| p.x >= 50 && p.x < 50 + width));
    }

    #[test]
    fn empty_text_has_zero_size() {
        let txt = Text::new("", TextStyle::default());
        assert_eq!((txt.width(), txt.height()), (0, 0));
    }

    #[test]
    fn text_style_scale_is_clamped_to_one() {
        assert_eq!(TextStyle::new(colors::WHITE, 0, 0).scale(), 1);
    }

    #[test]
    fn num_to_str_zero_sig_figs_uses_one() {
        assert_eq!(num_to_str(123.0, 0), num_to_str(123.0, 1));
    }

    #[test]
    fn label_limits_centered_returns_valid_bounds() {
        let style = TextStyle::default();
        let txt = Text::new("0", style);
        let center = Point::new(50, 50);
        let pos = TextPositioning::Centered(center);
        let label = Label::new(txt, pos);

        let limits = label.limits();
        // The limits should be centered around the center point
        assert!(limits.min().x < center.x);
        assert!(limits.min().y < center.y);
        assert!(limits.max().x > center.x);
        assert!(limits.max().y > center.y);
    }

    #[test]
    fn label_get_mask_centered_returns_points() {
        let style = TextStyle::default();
        let txt = Text::new("1", style);
        let pos = TextPositioning::Centered(Point::new(50, 50));
        let label = Label::new(txt, pos);

        let mask = label.get_mask().unwrap();
        assert!(!mask.is_empty());
        assert!(
            !mask[0].points.is_empty(),
            "Label mask should contain drawn points"
        );
    }

    #[test]
    fn num_to_str_within_range_gt_zero() {
        let number: f64 = 25.;
        let sig_figs: usize = 2;
        let num_str = num_to_str(number, sig_figs);

        assert_eq!(num_str, "25");
    }

    #[test]
    fn num_to_str_within_range_lt_zero() {
        let number: f64 = 0.25;
        let sig_figs: usize = 2;
        let num_str = num_to_str(number, sig_figs);

        assert_eq!(num_str, "0.25");
    }

    #[test]
    fn num_to_str_lt_min_range() {
        let number: f64 = 0.000250505;
        let sig_figs: usize = 2;
        let num_str = num_to_str(number, sig_figs);

        assert_eq!(num_str, "2.5e-4");
    }

    #[test]
    fn num_to_str_gt_max_range() {
        let number: f64 = 2_554_223.23;
        let sig_figs: usize = 2;
        let num_str = num_to_str(number, sig_figs);

        // rounded, not truncated
        assert_eq!(num_str, "2.6e6");
    }

    #[test]
    fn num_to_str_negative_and_rounding() {
        assert_eq!(num_to_str(-0.5, 1), "-0.5");
        assert_eq!(num_to_str(-0.25, 2), "-0.25");
        assert_eq!(num_to_str(0.96, 1), "1");
        assert_eq!(num_to_str(-1234.0, 2), "-1.2e3");
        assert_eq!(num_to_str(-0.0, 3), "0");
    }

    #[test]
    fn num_to_str_within_range_trailing_zeros() {
        let number: f64 = 0.2001;
        let sig_figs: usize = 2;
        let num_str = num_to_str(number, sig_figs);

        assert_eq!(num_str, "0.2");
    }

    #[test]
    fn labels_near_the_coordinate_limit_do_not_overflow() {
        for pos in [
            TextPositioning::Centered(Point::new(u32::MAX - 1, u32::MAX - 1)),
            TextPositioning::LeftAligned(Point::new(u32::MAX - 1, u32::MAX - 1)),
        ] {
            let label = Label::new(Text::new("12", TextStyle::default()), pos);
            assert_eq!(label.limits().max().x, u32::MAX);
            label.get_mask().unwrap();
        }
    }
}
