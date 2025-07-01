use super::{
    colors,
    common::{Drawable, IntConvertable, MaskPoints, UIntConvertable},
    limits::Limits,
    numbers,
    point::Point,
};
use crate::common::Result;
use rgb::RGB8;

#[derive(Debug, Clone)]
pub enum TextPositioning {
    Centered(Point<u32>),
    LeftAligned(Point<u32>),
    // to add...
}

impl TextPositioning {
    pub fn clone_with(&self, new_point: Point<u32>) -> Self {
        match self {
            Self::Centered(_) => Self::Centered(new_point),
            Self::LeftAligned(_) => Self::LeftAligned(new_point),
        }
    }

    pub fn point(&self) -> &Point<u32> {
        match self {
            Self::Centered(point) => point,
            Self::LeftAligned(point) => point,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextChar {
    value: char,
    bitmap: Vec<Vec<bool>>,
}

impl TextChar {
    pub fn new(value: char, style: &TextStyle) -> TextChar {
        let bitmap = numbers::get_bitmap(value, style);
        TextChar { value, bitmap }
    }

    pub fn width(&self) -> usize {
        self.bitmap.iter().map(|row| row.len()).max().unwrap()
    }

    pub fn height(&self) -> usize {
        self.bitmap.len()
    }

    pub fn value(&self) -> char {
        self.value
    }

    pub fn get_mask(&self, lower_left: Point<u32>, style: TextStyle) -> Result<Vec<MaskPoints>> {
        let mut points = Vec::new();
        for i in 0..self.height() {
            for j in 0..self.width() {
                if self.bitmap[i][j] {
                    let shift = Point::new(j as i32, i as i32);
                    let point = lower_left.convert_to_i32() + shift;
                    points.push(point.convert_to_u32());
                }
            }
        }
        let color = style.color;
        Ok(vec![MaskPoints { points, color }])
    }
}

#[derive(Debug, Clone)]
pub struct TextStyle {
    color: RGB8,
    scale: usize,
    padding: usize,
}

impl TextStyle {
    pub fn new(color: RGB8, scale: usize, padding: usize) -> TextStyle {
        if scale < 1 {
            panic!("Text scaling cannot be less than 1")
        }

        TextStyle {
            color,
            scale,
            padding,
        }
    }

    pub fn default() -> TextStyle {
        TextStyle {
            color: colors::BLACK,
            scale: 1,
            padding: 1,
        }
    }

    pub fn with_color(color: RGB8) -> TextStyle {
        TextStyle {
            color,
            scale: 1,
            padding: 1,
        }
    }

    pub fn color(&self) -> RGB8 {
        self.color
    }

    pub fn scale(&self) -> usize {
        self.scale
    }

    pub fn padding(&self) -> usize {
        self.padding
    }
}

#[derive(Debug, Clone)]
pub struct Text {
    style: TextStyle,
    chars: Vec<TextChar>,
    width: usize,
    height: usize,
}

impl Text {
    pub fn new(text: &str, style: TextStyle) -> Text {
        let chars = text
            .chars()
            .map(|c| TextChar::new(c, &style))
            .collect::<Vec<_>>();
        let width = chars.iter().fold(0usize, |acc, val| acc + val.width());
        let height = chars.iter().map(|c| c.height()).max().unwrap();

        Text {
            style,
            chars,
            width,
            height,
        }
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn chars(&self) -> &[TextChar] {
        &self.chars
    }

    pub fn from_number(number: f64, sig_figs: usize, style: TextStyle) -> Text {
        if style.scale < 1 {
            panic!("Text scaling cannot be less than 1")
        }
        Text::new(&num_to_str(number, sig_figs), style)
    }
}

fn num_to_str(number: f64, sig_figs: usize) -> String {
    if sig_figs < 1 {
        panic!("Number of significant figures must be nonzero")
    }

    let min = 0.1_f64.powi(sig_figs as i32);
    let max = 10_f64.powi(sig_figs as i32);

    let mut trunc_str = Vec::new();
    let mut sig_fig_count = 0;

    let num_str = if min < number.abs() && number.abs() < max {
        number.to_string()
    } else {
        format!("{number:e}")
    };

    let is_leading_zero = |s: &str, ind: usize| ind == 0 && s.chars().nth(ind).unwrap() == '0';

    // get the first part of the number, up to the required number of sig figs or the
    // scientific exponent (whichever comes first)
    for (ind, c) in num_str.chars().enumerate() {
        if sig_fig_count == sig_figs || c == 'e' {
            break;
        }
        trunc_str.push(c);
        if !is_leading_zero(&num_str, ind) && c.is_ascii_digit() {
            sig_fig_count += 1;
        }
    }

    // for decimal numbers, remove any trailing zeros (these are technically sig figs but removing
    // them yields a cleaner graph)
    while trunc_str.contains(&'.') && trunc_str.len() > 1 {
        if trunc_str.last().unwrap() == &'0' {
            trunc_str.pop()
        } else {
            break;
        };
    }

    // add the scientific component, but only if it is nonzero
    if num_str.contains('e') && !num_str.ends_with("e0") {
        let mut end_str = Vec::new();
        let mut chars = num_str.chars();
        while let Some(c) = chars.next_back() {
            end_str.insert(0, c.clone());
            if c == 'e' {
                break;
            }
        }
        trunc_str.extend(&end_str);
    }

    String::from_iter(trunc_str)
}

#[derive(Debug, Clone)]
pub struct Label {
    txt: Text,
    pos: TextPositioning,
}

impl Label {
    pub fn new(txt: Text, pos: TextPositioning) -> Label {
        Label { txt, pos }
    }

    pub fn txt(&self) -> &Text {
        &self.txt
    }

    pub fn pos(&self) -> &TextPositioning {
        &self.pos
    }

    pub fn limits(&self) -> Limits<u32> {
        match &self.pos {
            TextPositioning::Centered(center) => {
                let height_shift = (self.txt.height as f64) / 2.;
                let width_shift = (self.txt.width as f64) / 2.;
                let min = *center - Point::new(width_shift, height_shift).floor();
                let max = *center + Point::new(width_shift, height_shift).ceil();
                Limits::new(min, max)
            }
            _ => panic!("Not implemented"),
        }
    }
}

impl Drawable for Label {
    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let mask_points = match &self.pos {
            TextPositioning::Centered(center) => {
                let height_shift: i32 = (self.txt.height / 2).try_into().unwrap();
                let width_shift: i32 = (self.txt.width / 2).try_into().unwrap();
                let mut masks = Vec::new();
                self.txt.chars.iter().fold(-width_shift, |acc, c| {
                    let char_lower_left = center.convert_to_i32() + Point::new(acc, -height_shift);
                    masks.extend(
                        c.get_mask(char_lower_left.convert_to_u32(), self.txt.style.clone())
                            .unwrap(),
                    );
                    let char_width: i32 = c.width().try_into().unwrap();
                    acc + char_width
                });
                masks
            }
            _ => panic!("Not implemented"),
        };
        Ok(mask_points)
    }
}

#[cfg(test)]
mod test {
    use super::num_to_str;

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

        assert_eq!(num_str, "2.5e6");
    }

    #[test]
    fn num_to_str_within_range_trailing_zeros() {
        let number: f64 = 0.2001;
        let sig_figs: usize = 2;
        let num_str = num_to_str(number, sig_figs);

        assert_eq!(num_str, "0.2");
    }
}
