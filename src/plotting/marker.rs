use super::{
    colors,
    common::{DrawPositioning, Drawable, MaskPoints},
    line::LineStyle,
    point::Point,
};
use crate::common::Result;
use rgb::RGB8;

#[derive(Debug, PartialEq)]
pub enum MarkerStyle {
    FilledSquare {
        line_style: Option<LineStyle>,
        color: RGB8,
        size: u32,
    },
    HollowSquare {
        line_style: LineStyle,
        size: u32,
    },
}

impl MarkerStyle {
    pub const fn default() -> MarkerStyle {
        Self::FilledSquare {
            line_style: None,
            color: colors::WHITE,
            size: 0,
        }
    }

    pub const fn default_with_size(size: u32) -> MarkerStyle {
        Self::FilledSquare {
            line_style: None,
            color: colors::WHITE,
            size,
        }
    }
}

impl Drawable for MarkerStyle {
    fn bounding_width(&self) -> u32 {
        match self {
            Self::FilledSquare {
                line_style: _,
                color: _,
                size,
            } => size.clone(),
            Self::HollowSquare {
                line_style: _,
                size,
            } => size.clone(),
        }
    }

    fn bounding_height(&self) -> u32 {
        match self {
            Self::FilledSquare {
                line_style: _,
                color: _,
                size,
            } => size.clone(),
            Self::HollowSquare {
                line_style: _,
                size,
            } => size.clone(),
        }
    }

    fn get_mask(&self, pos: DrawPositioning) -> Result<Vec<MaskPoints>> {
        let mask_points = match pos {
            DrawPositioning::CenteredAt(center) => match self {
                MarkerStyle::FilledSquare {
                    line_style,
                    color,
                    size,
                } => {
                    let points = (center.x - size..=center.x + size)
                        .flat_map(|x| {
                            (center.y - size..=center.y + size).map(move |y| Point { x, y })
                        })
                        .collect::<Vec<Point<u32>>>();
                    vec![MaskPoints {
                        points,
                        color: color.clone(),
                    }]
                }
                _ => panic!("Not implemented!"),
            },
            _ => panic!("Not implemented!"),
        };
        Ok(mask_points)
    }
}
