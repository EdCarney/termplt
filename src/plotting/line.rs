use super::{
    colors,
    common::{
        Convertable, Drawable, FloatConvertable, Graphable, IntConvertable, MaskPoints, Scalable,
        Shiftable,
    },
    limits::Limits,
    line_positioning::LinePositioning,
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

impl Line<u32> {
    /// Gets drawable limits for the line.
    pub fn drawable_limits(&self) -> Limits<u32> {
        let limits = self.limits();
        let min = *limits.min() - self.style.thickness();
        let max = *limits.max() + self.style.thickness();
        Limits::new(min, max)
    }

    // Gets the full point set between the start and end of the line. Note that this does not
    // take into account empy space for dashed lines. Additionally this fn assumes the line is
    // already in a plottable space.
    pub fn full_drawable_points(&self) -> Vec<Point<u32>> {
        match self.positioning {
            LinePositioning::Vertical { .. } | LinePositioning::Horizontal { .. } => {
                Point::limit_range(self.positioning.limits())
            }
            LinePositioning::BetweenPoints { start, end } => {
                let m = (end.y - start.y).convert_to_f64() / (end.x - start.x).convert_to_f64();
                let b = start.y.convert_to_f64() - start.x.convert_to_f64() * m;

                // get iteration size; start w/ x step size of 1 and decrease until the dist b/w
                // two subsequent points is <= 1
                let p1 = start.convert_to_f64();
                let mut p2 = end.convert_to_f64();
                let mut x_step = (p2.x - p1.x).convert_to_f64();
                while !self.limits().contains(&p2.round()) || p1.dist(&p2) > 1. {
                    x_step /= 2.;
                    let x = p1.x + x_step;
                    let y = m * x + b;
                    p2 = Point::new(x, y);
                }

                let mut points = Vec::new();
                let num_iter = ((end.x - start.x).convert_to_f64() / x_step)
                    .floor()
                    .convert_to_u32();
                for i in 1..num_iter {
                    let step = x_step * i.convert_to_f64();
                    let x = p1.x + step;
                    let y = m * x + b;
                    let point = Point::new(x, y).convert_to_u32();

                    // only add points that are not already added
                    if !points.iter().any(|&p| p == point) {
                        points.push(point);
                    }
                }
                points
            }
        }
    }
}

impl<T: IntConvertable + Graphable> Drawable for Line<T> {
    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let flat_line_fn = |thickness: u32, pos: &LinePositioning<T>| -> Vec<Point<u32>> {
            let mut points = Vec::new();
            let shift_start = -1 * thickness as i32;
            let shift_end = thickness as i32;
            for shift in shift_start..=shift_end {
                let shift_point = match pos {
                    LinePositioning::Vertical { .. } => Point::new(shift, 0),
                    LinePositioning::Horizontal { .. } => Point::new(0, shift),
                    _ => panic!("Invalid line positioning type for flat line fn: {pos:?}"),
                };
                points.extend(
                    self.convert_to_u32()
                        .full_drawable_points()
                        .iter()
                        .map(|&p| (p.convert_to_i32() + shift_point).convert_to_u32()),
                );
            }
            points
        };
        let mask_points = match self.style {
            LineStyle::Solid { color, thickness } => {
                let points = match self.positioning {
                    LinePositioning::Vertical { .. } | LinePositioning::Horizontal { .. } => {
                        flat_line_fn(thickness, &self.positioning)
                    }
                    LinePositioning::BetweenPoints { .. } => {
                        self.convert_to_u32().full_drawable_points()
                    }
                };

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
