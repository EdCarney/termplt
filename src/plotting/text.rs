use super::{
    colors,
    common::{Drawable, IntConvertable, MaskPoints, UIntConvertable},
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
    bitmap: Vec<Vec<i32>>,
}

impl TextChar {
    pub fn new(value: char) -> TextChar {
        let bitmap = match value {
            '0' => vec![
                vec![0, 1, 1, 1, 0],
                vec![1, 1, 0, 1, 1],
                vec![1, 0, 0, 0, 1],
                vec![1, 0, 0, 0, 1],
                vec![1, 0, 0, 0, 1],
                vec![1, 0, 0, 0, 1],
                vec![1, 0, 0, 0, 1],
                vec![1, 0, 0, 0, 1],
                vec![1, 1, 0, 1, 1],
                vec![0, 1, 1, 1, 0],
            ],
            _ => panic!("Bitmap not defined for character: '{value}'"),
        };
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
                if self.bitmap[i][j] == 1 {
                    let shift = Point::new(j as i32, i as i32);
                    let point = top_left.convert_to_i32() + shift;
                    points.push(point);
                }
            }
        }
        let points = self
            .bitmap
            .iter()
            .flatten()
            .map(|&shift| (top_left.convert_to_i32() + shift).convert_to_u32())
            .collect::<Vec<_>>();
        let color = style.color;
        Ok(vec![MaskPoints { points, color }])
    }
}

#[derive(Debug, Clone)]
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
        if style.size < 5 {
            panic!("Text size cannot be less than 5")
        }

        let chars = text.chars().map(|c| TextChar::new(c)).collect::<Vec<_>>();
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
                let height_shift: i32 = self.height.try_into().unwrap();
                let height_shift = height_shift / 2;
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
