use super::{
    colors,
    common::{Drawable, IntConvertable, MaskPoints, UIntConvertable},
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
    pub fn new(text: String, style: TextStyle) -> Text {
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

    pub fn from_number(number: f64, sig_figs: u8, style: TextStyle) -> Text {
        if style.scale < 1 {
            panic!("Text scaling cannot be less than 1")
        }
        if sig_figs < 1 {
            panic!("Number of significant figures must be nonzero")
        }

        let full_sci = format!("{number:e}");
        let mut trunc_sci = Vec::new();
        let mut sig_fig_count = 0;

        // get the first part of the number, up to the required number of sig figs or the
        // scientific exponent (whichever comes first)
        for c in full_sci.chars() {
            if sig_fig_count == sig_figs || c == 'e' {
                break;
            }
            trunc_sci.push(c);
            if c.is_ascii_digit() {
                sig_fig_count += 1;
            }
        }

        // add the scientific component, but only if it is nonzero
        if !full_sci.ends_with("e0") {
            let mut end_str = Vec::new();
            let mut chars = full_sci.chars();
            while let Some(c) = chars.next_back() {
                end_str.insert(0, c.clone());
                if c == 'e' {
                    break;
                }
            }
            trunc_sci.extend(&end_str);
        }

        Text::new(String::from_iter(trunc_sci), style)
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn width(&self) -> usize {
        self.width
    }
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
