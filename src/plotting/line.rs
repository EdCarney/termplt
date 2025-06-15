use super::{
    colors,
    common::{
        Convertable, Drawable, FloatConvertable, Graphable, IntConvertable, MaskPoints, Scalable,
        Shiftable,
    },
    limits::Limits,
    point::Point,
};
use crate::{common::Result, plotting::common::UIntConvertable};
use rgb::RGB8;

#[derive(Debug, PartialEq, Copy, Clone)]
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

impl<T, U> Scalable<T, U> for LinePositioning<T>
where
    T: FloatConvertable + Graphable,
    U: FloatConvertable + Graphable,
{
    type ScaleTo = LinePositioning<f64>;
    fn scale_to(self, old_limits: &Limits<T>, new_limits: &Limits<U>) -> Self::ScaleTo {
        let limits = self.limits().scale_to(old_limits, new_limits);
        let start = *limits.min();
        let end = *limits.max();
        let length = end.dist(&start);
        let scaled_pos = match self {
            LinePositioning::Vertical { .. } => LinePositioning::Vertical { start, length },
            LinePositioning::Horizontal { .. } => LinePositioning::Horizontal { start, length },
            LinePositioning::BetweenPoints { .. } => LinePositioning::BetweenPoints { start, end },
        };
        scaled_pos
    }
}

impl<T> Shiftable<T> for LinePositioning<T>
where
    T: FloatConvertable + Graphable,
{
    fn shift_by(self, amount: Point<T>) -> Self {
        let limits = self.limits().shift_by(amount);
        let start = *limits.min();
        let end = *limits.max();
        match self {
            LinePositioning::Vertical { length, .. } => LinePositioning::Vertical { start, length },
            LinePositioning::Horizontal { length, .. } => {
                LinePositioning::Horizontal { start, length }
            }
            LinePositioning::BetweenPoints { .. } => LinePositioning::BetweenPoints { start, end },
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

impl<T, U> Scalable<T, U> for Line<T>
where
    T: FloatConvertable + Graphable,
    U: FloatConvertable + Graphable,
{
    type ScaleTo = Line<f64>;
    fn scale_to(self, old_limits: &Limits<T>, new_limits: &Limits<U>) -> Self::ScaleTo {
        let style = self.style;
        let positioning = self.positioning.scale_to(old_limits, new_limits);
        Line { style, positioning }
    }
}

impl<T> Shiftable<T> for Line<T>
where
    T: FloatConvertable + Graphable,
{
    fn shift_by(self, amount: Point<T>) -> Self {
        let style = self.style;
        let positioning = self.positioning.shift_by(amount);
        Line { style, positioning }
    }
}

impl<T: IntConvertable + Graphable> Line<T> {
    /// Gets drawable limits for the line.
    pub fn drawable_limits(&self) -> Limits<u32> {
        let limits = self.limits().convert_to_u32();
        let min = *limits.min() - self.style.thickness();
        let max = *limits.max() + self.style.thickness();
        Limits::new(min, max)
    }

    // Gets the full point set between the start and end of the line. Note that this does not
    // take into account empy space for dashed lines.
    pub fn full_drawable_points(&self) -> Vec<Point<u32>> {
        Point::limit_range(self.positioning.limits())
    }
}

impl<T: IntConvertable + Graphable> Drawable for Line<T> {
    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let shift_point_fn = match self.positioning {
            LinePositioning::Vertical { .. } => |amount: i32| Point::new(amount, 0),
            LinePositioning::Horizontal { .. } => |amount: i32| Point::new(0, amount),
            LinePositioning::BetweenPoints { .. } => todo!(),
        };
        let mask_points = match self.style {
            LineStyle::Solid { color, thickness } => {
                let mut points = Vec::new();

                let shift_start = -1 * thickness as i32;
                let shift_end = thickness as i32;
                for shift in shift_start..=shift_end {
                    let shift_point = shift_point_fn(shift);
                    points.extend(
                        self.full_drawable_points()
                            .iter()
                            .map(|&p| (p.convert_to_i32() + shift_point).convert_to_u32()),
                    );
                }

                vec![MaskPoints {
                    points,
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
