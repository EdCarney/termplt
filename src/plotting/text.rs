use super::{
    colors,
    common::{Drawable, IntConvertable, MaskPoints, UIntConvertable},
    point::Point,
};
use crate::common::Result;
use rgb::RGB8;

pub enum TextPositioning {
    Centered(Point<u32>),
    LeftAligned(Point<u32>),
    // to add...
}

pub struct TextChar {
    value: char,
    bitmap: Vec<Vec<i32>>,
    center: Point<u32>,
    style: TextStyle,
}

impl TextChar {
    pub fn new(value: char, center: Point<u32>, style: TextStyle) -> TextChar {
        let bitmap = match value {
            '0' => vec![vec![0, 0, 1, 0, 0], vec![]],
            _ => panic!("Bitmap not defined for character: '{value}'"),
        };
        TextChar {
            value,
            bitmap,
            center,
            style,
        }
    }
}

impl Drawable for TextChar {
    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let points = self
            .bitmap
            .iter()
            .flatten()
            .map(|&shift| (self.center.convert_to_i32() + shift).convert_to_u32())
            .collect::<Vec<_>>();
        let color = self.style.color;
        Ok(vec![MaskPoints { points, color }])
    }
}

pub struct TextStyle {
    pub color: RGB8,
    pub size: u32,
}

impl TextStyle {
    pub fn default() -> TextStyle {
        TextStyle {
            color: colors::BLACK,
            size: 5,
        }
    }
}

pub struct Text {
    text: String,
    style: TextStyle,
    positioning: TextPositioning,
}

impl Text {
    pub fn new(text: String, style: TextStyle, positioning: TextPositioning) -> Text {
        if style.size < 5 {
            panic!("Text size cannot be less than 5")
        }
        Text {
            text,
            style,
            positioning,
        }
    }
}

impl Drawable for Text {
    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        todo!()
    }
}
