use super::{
    colors,
    common::{Convertable, Drawable, Graphable, MaskPoints, UIntConvertable},
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

#[derive(Debug, Clone)]
pub enum LinePositioning<T: Graphable> {
    Horizontal { start: Point<T>, length: T },
    Vertical { start: Point<T>, length: T },
    BetweenPoints { start: Point<T>, end: Point<T> },
}

impl<T: Graphable> LinePositioning<T> {
    pub fn limits(&self) -> Limits<T> {
        let (min, max) = match self {
            LinePositioning::Horizontal { start, length } => {
                let start = start.clone();
                let end = Point::new(start.x + *length, start.y);
                (start, end)
            }
            LinePositioning::Vertical { start, length } => {
                let start = start.clone();
                let end = Point::new(start.x, start.y + *length);
                (start, end)
            }
            LinePositioning::BetweenPoints { start, end } => {
                // for thickness b/w points, assume that the thickness will not go outside the
                // limits defined by the two points
                (start.clone(), end.clone())
            }
        };
        Limits::new(min, max)
    }
}

impl<T: Graphable, U: Graphable> Convertable<U> for LinePositioning<T> {
    type ConvertTo = LinePositioning<U>;
    fn convert_to(&self, convert_fn: unsafe fn(f64) -> U) -> Self::ConvertTo {
        match &self {
            LinePositioning::Horizontal { start, length } => {
                let start = start.convert_to(convert_fn);
                let length = length.convert_to(convert_fn);
                LinePositioning::Horizontal { start, length }
            }
            LinePositioning::Vertical { start, length } => {
                let start = start.convert_to(convert_fn);
                let length = length.convert_to(convert_fn);
                LinePositioning::Vertical { start, length }
            }
            LinePositioning::BetweenPoints { start, end } => {
                let start = start.convert_to(convert_fn);
                let end = end.convert_to(convert_fn);
                LinePositioning::BetweenPoints { start, end }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Line<T: Graphable> {
    style: LineStyle,
    positioning: LinePositioning<T>,
}

impl<T: Graphable, U: Graphable> Convertable<U> for Line<T> {
    type ConvertTo = Line<U>;
    fn convert_to(&self, convert_fn: unsafe fn(f64) -> U) -> Self::ConvertTo {
        let style = self.style().clone();
        let positioning = self.positioning.convert_to(convert_fn);
        Line { style, positioning }
    }
}

impl<T: Graphable> Line<T> {
    pub fn new(positioning: LinePositioning<T>, style: LineStyle) -> Line<T> {
        Line { style, positioning }
    }

    pub fn default(positioning: LinePositioning<T>) -> Line<T> {
        Line {
            style: LineStyle::default(),
            positioning,
        }
    }

    pub fn style(&self) -> &LineStyle {
        &self.style
    }

    pub fn limits(&self) -> Limits<T> {
        self.positioning.limits()
    }
}

impl<T: UIntConvertable + Graphable> Line<T> {
    /// Gets bounding limits for the line.
    pub fn drawable_limits(&self) -> Limits<u32> {
        let limits = self.positioning.limits().convert_to_u32();
        let min = *limits.min() - self.style.thickness();
        let max = *limits.max() + self.style.thickness();
        Limits::new(min, max)
    }

    // Gets the full point set between the start and end of the line. Note that this does not
    // take into account empy space for dashed lines.
    pub fn full_drawable_points(&self) -> Vec<Point<u32>> {
        match self.positioning {
            LinePositioning::Vertical {
                start: _,
                length: _,
            }
            | LinePositioning::Horizontal {
                start: _,
                length: _,
            } => Point::limit_range(self.drawable_limits()),
            LinePositioning::BetweenPoints { start, end } => {
                todo!()
            }
        }
    }
}

impl Drawable for Line<u32> {
    fn bounding_height(&self) -> u32 {
        let limits = self.drawable_limits();
        (*limits.max() - *limits.min()).convert_to_u32().y
    }

    fn bounding_width(&self) -> u32 {
        let limits = self.drawable_limits();
        (*limits.max() - *limits.min()).convert_to_u32().x
    }

    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let mask_points = match self.style {
            LineStyle::Solid {
                color,
                thickness: _,
            } => {
                vec![MaskPoints {
                    points: self.full_drawable_points(),
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
