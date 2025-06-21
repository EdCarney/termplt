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
                "  000000  ",
                "0000  0000",
                "00      00",
                "00      00",
                "00  00  00",
                "00  00  00",
                "00      00",
                "00      00",
                "0000  0000",
                "  000000  ",
            ],
            '1' => vec![
                "    11    ",
                "  1111    ",
                "11  11    ",
                "    11    ",
                "    11    ",
                "    11    ",
                "    11    ",
                "    11    ",
                "    11    ",
                "1111111111",
            ],
            '2' => vec![
                "  222222  ",
                "2222  2222",
                "22    2222",
                "    2222  ",
                "    2222  ",
                "  2222    ",
                "  2222    ",
                "2222      ",
                "2222      ",
                "2222222222",
            ],
            '3' => vec![
                "3333333333",
                "      3333",
                "    3333  ",
                "  3333    ",
                " 33333    ",
                "    3333  ",
                "      3333",
                "33      33",
                "3333  3333",
                "  333333  ",
            ],
            _ => panic!("Bitmap not defined for character: '{value}'"),
        };

        // note that bitmaps are written to be human-readable; they need to be modified to be
        // printed; this includes correcting the aspect ratio (every other row element is skipped
        // to ensure the aspect ratio is 1:2); also the string is converted to a vec of i32
        let mut bitmap = bitmap
            .iter()
            .map(|row| {
                row.to_string()
                    .chars()
                    .map(|x| if x == ' ' { 0 } else { 1 })
                    .step_by(2)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        // the rows of the bitmap must also be reversed to ensure that lower indices are the bottom
        // of the coordinates
        bitmap.reverse();

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
