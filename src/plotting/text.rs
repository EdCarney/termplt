use super::{
    colors,
    common::{Drawable, IntConvertable, MaskPoints, UIntConvertable},
    numbers::get_bitmap,
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
    pub fn new(value: char, scale: u8) -> TextChar {
        let bitmap = get_bitmap(value, scale);
        TextChar { value, bitmap }
    }

    pub fn width(&self) -> usize {
        self.bitmap.first().unwrap().len()
    }

    pub fn height(&self) -> usize {
        self.bitmap.len()
    }

    pub fn get_mask(&self, top_left: Point<u32>, style: TextStyle) -> Result<Vec<MaskPoints>> {
        let mut points = Vec::new();
        for i in 0..self.height() {
            for j in 0..self.width() {
                if self.bitmap[i][j] {
                    let shift = Point::new(j as i32, i as i32);
                    let point = top_left.convert_to_i32() + shift;
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
    pub color: RGB8,
    pub scale: u8,
}

impl TextStyle {
    pub fn new(color: RGB8, scale: u8) -> TextStyle {
        TextStyle { color, scale }
    }

    pub fn default() -> TextStyle {
        TextStyle {
            color: colors::BLACK,
            scale: 5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Text {
    text: String,
    style: TextStyle,
    positioning: TextPositioning,
    chars: Vec<TextChar>,
    width: usize,
    height: usize,
}

impl Text {
    pub fn new(text: String, style: TextStyle, positioning: TextPositioning) -> Text {
        if style.scale < 1 {
            panic!("Text scaling cannot be less than 1")
        }

        let chars = text
            .chars()
            .map(|c| TextChar::new(c, style.scale))
            .collect::<Vec<_>>();
        let width = chars.iter().fold(0usize, |acc, val| acc + val.width());
        let height = chars.iter().map(|c| c.height()).max().unwrap();

        Text {
            text,
            style,
            positioning,
            chars,
            width,
            height,
        }
    }
}

impl Drawable for Text {
    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let mask_points = match self.positioning {
            TextPositioning::Centered(center) => {
                let height_shift: i32 = (self.height / 2).try_into().unwrap();
                let shift: i32 = (self.width / 2).try_into().unwrap();
                let mut masks = Vec::new();
                self.chars.iter().fold(-shift, |acc, c| {
                    let char_top_left = center.convert_to_i32() + Point::new(acc, height_shift);
                    masks.extend(
                        c.get_mask(char_top_left.convert_to_u32(), self.style.clone())
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
