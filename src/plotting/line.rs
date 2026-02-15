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
    fn convert_to(&self, convert_fn: fn(f64) -> U) -> Self::ConvertTo {
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

impl Line<i32> {
    /// Gets drawable limits for the line.
    pub fn drawable_limits(&self) -> Limits<u32> {
        let limits = self.limits().convert_to_u32();
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
                // Bresenham's line algorithm — handles all orientations including
                // vertical, horizontal, steep, shallow, and any direction.
                let mut x0 = start.x;
                let mut y0 = start.y;
                let x1 = end.x;
                let y1 = end.y;

                let dx = (x1 - x0).abs();
                let dy = -(y1 - y0).abs();
                let sx = if x0 < x1 { 1 } else { -1 };
                let sy = if y0 < y1 { 1 } else { -1 };
                let mut err = dx + dy;

                let mut points = Vec::new();
                loop {
                    let point = Point::new(x0, y0).convert_to_u32();
                    if points.last() != Some(&point) {
                        points.push(point);
                    }

                    if x0 == x1 && y0 == y1 {
                        break;
                    }

                    let e2 = 2 * err;
                    if e2 >= dy {
                        err += dy;
                        x0 += sx;
                    }
                    if e2 <= dx {
                        err += dx;
                        y0 += sy;
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
                    self.convert_to_i32()
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
                        self.convert_to_i32().full_drawable_points()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "not yet implemented")]
    fn dashed_line_get_mask_panics_with_todo() {
        let style = LineStyle::Dashed {
            color: colors::WHITE,
            thickness: 0,
        };
        let pos = LinePositioning::Horizontal {
            start: Point::new(0, 0),
            length: 10,
        };
        let line: Line<i32> = Line::new(pos, style);
        let _ = line.get_mask();
    }

    #[test]
    fn solid_horizontal_line_get_mask_returns_points() {
        let style = LineStyle::Solid {
            color: colors::WHITE,
            thickness: 0,
        };
        let pos = LinePositioning::Horizontal {
            start: Point::new(0, 5),
            length: 10,
        };
        let line: Line<i32> = Line::new(pos, style);
        let mask = line.get_mask().unwrap();
        assert!(!mask.is_empty());
        assert!(!mask[0].points.is_empty());
    }

    #[test]
    fn solid_vertical_line_get_mask_returns_points() {
        let style = LineStyle::Solid {
            color: colors::WHITE,
            thickness: 0,
        };
        let pos = LinePositioning::Vertical {
            start: Point::new(5, 0),
            length: 10,
        };
        let line: Line<i32> = Line::new(pos, style);
        let mask = line.get_mask().unwrap();
        assert!(!mask.is_empty());
        assert!(!mask[0].points.is_empty());
    }

    #[test]
    fn solid_between_points_left_to_right_get_mask_returns_points() {
        let style = LineStyle::Solid {
            color: colors::WHITE,
            thickness: 0,
        };
        let pos = LinePositioning::BetweenPoints {
            start: Point::new(0, 0),
            end: Point::new(10, 10),
        };
        let line: Line<i32> = Line::new(pos, style);
        let mask = line.get_mask().unwrap();
        assert!(!mask.is_empty());
        assert!(!mask[0].points.is_empty());
    }

    #[test]
    fn full_drawable_points_vertical_line_should_not_panic() {
        // Vertical line: start.x == end.x, so slope calculation divides by zero.
        // This test documents the current bug — it should produce points, not panic.
        let pos = LinePositioning::BetweenPoints {
            start: Point::new(5, 0),
            end: Point::new(5, 10),
        };
        let line = Line::new(pos, LineStyle::default());
        let points = line.full_drawable_points();
        assert!(!points.is_empty(), "Vertical line should produce points");
    }

    #[test]
    fn full_drawable_points_right_to_left_produces_points() {
        // Right-to-left line: start.x > end.x.
        // After fix, points are generated left-to-right then reversed.
        let pos = LinePositioning::BetweenPoints {
            start: Point::new(10, 0),
            end: Point::new(0, 10),
        };
        let line = Line::new(pos, LineStyle::default());
        let points = line.full_drawable_points();
        assert!(!points.is_empty(), "Right-to-left line should produce points");
        // First point should be near start (10, 0), last near end (0, 10)
        let first = points.first().unwrap();
        let last = points.last().unwrap();
        assert!(
            first.x >= last.x,
            "RTL line points should go from higher x to lower x, got first.x={} last.x={}",
            first.x,
            last.x
        );
    }

    #[test]
    fn full_drawable_points_single_point_line() {
        // Start == end: zero-length line.
        let pos = LinePositioning::BetweenPoints {
            start: Point::new(5, 5),
            end: Point::new(5, 5),
        };
        let line = Line::new(pos, LineStyle::default());
        let points = line.full_drawable_points();
        // A zero-length line should produce at most 1 point (or 0 is acceptable)
        assert!(points.len() <= 1, "Single-point line should produce 0 or 1 points");
    }

    #[test]
    fn full_drawable_points_left_to_right_diagonal() {
        // Normal case: left-to-right diagonal.
        let pos = LinePositioning::BetweenPoints {
            start: Point::new(0, 0),
            end: Point::new(10, 10),
        };
        let line = Line::new(pos, LineStyle::default());
        let points = line.full_drawable_points();
        assert!(!points.is_empty(), "Diagonal line should produce points");
        // Should cover the full range
        let min_x = points.iter().map(|p| p.x).min().unwrap();
        let max_x = points.iter().map(|p| p.x).max().unwrap();
        assert!(max_x > min_x, "Points should span a range of x values");
    }

    #[test]
    fn full_drawable_points_steep_line_no_gaps() {
        // Steep line (slope > 1): should still produce continuous points.
        let pos = LinePositioning::BetweenPoints {
            start: Point::new(0, 0),
            end: Point::new(3, 30),
        };
        let line = Line::new(pos, LineStyle::default());
        let points = line.full_drawable_points();
        assert!(
            points.len() >= 3,
            "Steep line should produce at least as many points as the x range"
        );
    }

    #[test]
    fn solid_line_with_thickness_produces_more_points() {
        let thin_style = LineStyle::Solid {
            color: colors::WHITE,
            thickness: 0,
        };
        let thick_style = LineStyle::Solid {
            color: colors::WHITE,
            thickness: 2,
        };
        let pos = LinePositioning::Horizontal {
            start: Point::new(0, 5),
            length: 10,
        };
        let thin_line: Line<i32> = Line::new(pos, thin_style);
        let thick_line: Line<i32> = Line::new(pos, thick_style);

        let thin_count = thin_line.get_mask().unwrap()[0].points.len();
        let thick_count = thick_line.get_mask().unwrap()[0].points.len();
        assert!(
            thick_count > thin_count,
            "Thick line ({thick_count} points) should have more points than thin ({thin_count})"
        );
    }
}
