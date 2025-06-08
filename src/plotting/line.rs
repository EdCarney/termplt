use super::{
    colors,
    common::{Drawable, MaskPoints},
    limits::Limits,
    point::Point,
};
use crate::common::Result;
use rgb::RGB8;

#[derive(Debug, PartialEq, Clone)]
pub enum LineStyle {
    Solid { color: RGB8, thickness: u32 },
    Dashed { color: RGB8, thickness: u32 },
}

impl LineStyle {
    pub const fn default() -> LineStyle {
        Self::Solid {
            color: colors::WHITE,
            thickness: 0,
        }
    }

    pub const fn default_with_thickness(thickness: u32) -> LineStyle {
        Self::Solid {
            color: colors::WHITE,
            thickness,
        }
    }

    pub fn thickness(&self) -> u32 {
        match self {
            LineStyle::Solid {
                color: _,
                thickness,
            } => *thickness,
            LineStyle::Dashed {
                color: _,
                thickness,
            } => *thickness,
        }
    }

    pub fn color(&self) -> RGB8 {
        match self {
            LineStyle::Solid {
                color,
                thickness: _,
            } => *color,
            LineStyle::Dashed {
                color,
                thickness: _,
            } => *color,
        }
    }
}

#[derive(Debug)]
pub enum LinePositioning {
    Horizontal { start: Point<u32>, length: u32 },
    Vertical { start: Point<u32>, length: u32 },
    BetweenPoints { start: Point<u32>, end: Point<u32> },
}

#[derive(Debug)]
pub struct Line {
    style: LineStyle,
    positioning: LinePositioning,
}

impl Line {
    pub fn new(positioning: LinePositioning, style: LineStyle) -> Line {
        Line { style, positioning }
    }

    pub fn default(positioning: LinePositioning) -> Line {
        Line {
            style: LineStyle::default(),
            positioning,
        }
    }

    /// Gets bounding limits for the line.
    pub fn limits(&self) -> Limits<u32> {
        let thickness = self.style.thickness();
        let (from, to) = match self.positioning {
            LinePositioning::Horizontal { start, length } => {
                let from = start - Point::new(0, thickness);
                let to = start + Point::new(length, thickness);
                (from, to)
            }
            LinePositioning::Vertical { start, length } => {
                let from = start - Point::new(thickness, 0);
                let to = start + Point::new(thickness, length);
                (from, to)
            }
            LinePositioning::BetweenPoints { start, end } => {
                // for thickness b/w points, assume that the thickness will not go outside the
                // limits defined by the two points
                (start, end)
            }
        };
        Limits::new(from, to)
    }

    // Gets the full point set between the start and end of the line. Note that this does not
    // take into account empy space for dashed lines.
    fn full_points(&self) -> Vec<Point<u32>> {
        match self.positioning {
            LinePositioning::Vertical {
                start: _,
                length: _,
            }
            | LinePositioning::Horizontal {
                start: _,
                length: _,
            } => Point::<u32>::limit_range(self.limits()),
            LinePositioning::BetweenPoints { start, end } => {
                todo!()
            }
        }
    }
}

impl Drawable for Line {
    fn bounding_height(&self) -> u32 {
        let limits = self.limits();
        (*limits.max() - *limits.min()).y
    }

    fn bounding_width(&self) -> u32 {
        let limits = self.limits();
        (*limits.max() - *limits.min()).x
    }

    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let mask_points = match self.style {
            LineStyle::Solid {
                color,
                thickness: _,
            } => {
                vec![MaskPoints {
                    points: self.full_points(),
                    color: color.clone(),
                }]
            }
            LineStyle::Dashed {
                color,
                thickness: _,
            } => todo!(),
        };
        Ok(mask_points)
    }
}
