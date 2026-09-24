use super::{
    canvas::Canvas,
    colors,
    common::{Convertable, Drawable, Graphable, IntConvertable, MaskPoints},
    line_positioning::LinePositioning,
    point::Point,
};
use crate::{common::Result, plotting::common::UIntConvertable};
use rgb::RGB8;

/// How a line is drawn. A thickness of 0 is one pixel wide; each step adds a pixel on both
/// sides.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum LineStyle {
    /// A continuous line.
    Solid {
        /// Line color.
        color: RGB8,
        /// Extra pixels on each side of a one-pixel line.
        thickness: u32,
    },
    /// A dashed line (6 pixels on, 4 off).
    Dashed {
        /// Line color.
        color: RGB8,
        /// Extra pixels on each side of a one-pixel line.
        thickness: u32,
    },
}

impl Default for LineStyle {
    /// A thin solid white line.
    fn default() -> LineStyle {
        LineStyle::solid(colors::WHITE, 0)
    }
}

impl LineStyle {
    /// A solid line. A thickness of 0 is one pixel wide; each step adds a pixel on both sides.
    pub const fn solid(color: RGB8, thickness: u32) -> LineStyle {
        LineStyle::Solid { color, thickness }
    }

    /// A dashed line (6 pixels on, 4 off).
    pub const fn dashed(color: RGB8, thickness: u32) -> LineStyle {
        LineStyle::Dashed { color, thickness }
    }

    /// The line's thickness (see [`LineStyle::solid`]).
    pub fn thickness(&self) -> u32 {
        match self {
            LineStyle::Solid { thickness, .. } | LineStyle::Dashed { thickness, .. } => *thickness,
        }
    }

    /// The line's color.
    pub fn color(&self) -> RGB8 {
        match self {
            LineStyle::Solid { color, .. } | LineStyle::Dashed { color, .. } => *color,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Line<T: Graphable> {
    style: LineStyle,
    positioning: LinePositioning<T>,
}

impl<T: Graphable, U: Graphable> Convertable<U> for Line<T> {
    type ConvertTo = Line<U>;
    fn convert_to(&self, convert_fn: fn(f64) -> U) -> Self::ConvertTo {
        let style = *self.style();
        let positioning = self.positioning.convert_to(convert_fn);
        Line { style, positioning }
    }
}

impl<T: Graphable> Line<T> {
    pub(crate) fn new(positioning: LinePositioning<T>, style: LineStyle) -> Line<T> {
        Line { style, positioning }
    }

    pub(crate) fn style(&self) -> &LineStyle {
        &self.style
    }
}

impl Line<i32> {
    // Gets the full point set between the start and end of the line. Note that this does not
    // take into account empy space for dashed lines. Additionally this fn assumes the line is
    // already in a plottable space.
    pub(crate) fn full_drawable_points(&self) -> Vec<Point<u32>> {
        match self.positioning {
            LinePositioning::Vertical { .. } | LinePositioning::Horizontal { .. } => {
                Point::limit_range(self.positioning.limits())
            }
            LinePositioning::BetweenPoints { start, end } => {
                let mut points = Vec::new();
                bresenham(start, end, |p| points.push(p.convert_to_u32()));
                points
            }
        }
    }
}

/// Bresenham's line algorithm: calls `f` for every pixel from `start` to `end` in order. Handles
/// all orientations (vertical, horizontal, steep, shallow, any direction); consecutive pixels
/// always differ.
fn bresenham(start: Point<i32>, end: Point<i32>, mut f: impl FnMut(Point<i32>)) {
    let (mut x0, mut y0) = (start.x, start.y);
    let (x1, y1) = (end.x, end.y);

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        f(Point::new(x0, y0));
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
}

/// A line segment drawn straight into a canvas: the same pixels as
/// `Line::new(BetweenPoints { start, end }, style).get_mask()`, without the intermediate
/// allocations. `stamp` must be `LineStamp::new(style)`, and `last` the last pixel stamped with
/// it in this color (whose stamp is therefore fully painted), which is updated.
pub(crate) fn draw_segment(
    canvas: &mut Canvas,
    (start, end): (Point<u32>, Point<u32>),
    style: &LineStyle,
    stamp: &LineStamp,
    last: &mut Option<Point<i32>>,
) {
    let color = style.color();
    let dashed = matches!(style, LineStyle::Dashed { .. });
    let mut index = 0;
    bresenham(start.convert_to_i32(), end.convert_to_i32(), |p| {
        let on = !dashed || index % (DASH_ON + DASH_OFF) < DASH_ON;
        index += 1;
        if !on {
            return;
        }
        // the previous stamp is already painted, so a neighbouring pixel only needs the part of
        // its stamp that the previous one did not cover (and a repeated pixel needs nothing);
        // dense data revisits the same pixels many times
        let offsets = match *last {
            Some(prev) if prev == p => return,
            Some(prev) if (p.x - prev.x).abs() <= 1 && (p.y - prev.y).abs() <= 1 => {
                &stamp.steps[LineStamp::step_index(p.x - prev.x, p.y - prev.y)]
            }
            _ => &stamp.full,
        };
        *last = Some(p);
        for offset in offsets {
            let q = (p + *offset).convert_to_u32();
            canvas.put(q.x, q.y, color);
        }
    });
}

/// The pixels stamped at each point of a line: a disc of radius `thickness` (just the point for
/// a thickness of 0), plus, for each of the 8 steps to a neighbouring pixel, the part of the disc
/// that the disc at the previous pixel does not cover.
pub(crate) struct LineStamp {
    full: Vec<Point<i32>>,
    steps: [Vec<Point<i32>>; 9],
}

impl LineStamp {
    pub(crate) fn new(style: &LineStyle) -> LineStamp {
        let full = disc_offsets(style.thickness());
        let steps = std::array::from_fn(|i| {
            let (dx, dy) = (i as i32 % 3 - 1, i as i32 / 3 - 1);
            // offsets relative to the new pixel that are outside the disc around the old one
            full.iter()
                .copied()
                .filter(|o| !full.contains(&Point::new(o.x + dx, o.y + dy)))
                .collect()
        });
        LineStamp { full, steps }
    }

    /// Index into `steps` for a step of (`dx`, `dy`), each in -1..=1.
    fn step_index(dx: i32, dy: i32) -> usize {
        ((dy + 1) * 3 + (dx + 1)) as usize
    }
}

/// Offsets of all pixels within `radius` of the origin.
fn disc_offsets(radius: u32) -> Vec<Point<i32>> {
    let r = radius as i32;
    (-r..=r)
        .flat_map(|dx| (-r..=r).map(move |dy| Point::new(dx, dy)))
        .filter(|p| p.x * p.x + p.y * p.y <= r * r)
        .collect()
}

/// Number of pixels drawn, then skipped, along a dashed line.
const DASH_ON: usize = 6;
const DASH_OFF: usize = 4;

impl<T: IntConvertable + Graphable> Drawable for Line<T> {
    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let (color, thickness, dashed) = match self.style {
            LineStyle::Solid { color, thickness } => (color, thickness, false),
            LineStyle::Dashed { color, thickness } => (color, thickness, true),
        };

        // pixels along the line, ordered from start to end
        let mut path = self.convert_to_i32().full_drawable_points();
        if dashed {
            path = path
                .into_iter()
                .enumerate()
                .filter(|(i, _)| i % (DASH_ON + DASH_OFF) < DASH_ON)
                .map(|(_, p)| p)
                .collect();
        }

        let points = match self.positioning {
            LinePositioning::Vertical { .. } | LinePositioning::Horizontal { .. } => {
                // thickness is applied by drawing shifted copies on both sides of the line
                let mut points = Vec::with_capacity(path.len() * (2 * thickness as usize + 1));
                for shift in -(thickness as i32)..=thickness as i32 {
                    let shift_point = match self.positioning {
                        LinePositioning::Vertical { .. } => Point::new(shift, 0),
                        _ => Point::new(0, shift),
                    };
                    points.extend(
                        path.iter()
                            .map(|&p| (p.convert_to_i32() + shift_point).convert_to_u32()),
                    );
                }
                points
            }
            // thickness is applied by stamping a disc of radius `thickness` at every pixel, which
            // also gives round joins where consecutive segments meet
            LinePositioning::BetweenPoints { .. } if thickness > 0 => {
                let stamp = disc_offsets(thickness);
                let mut points = Vec::with_capacity(path.len() * stamp.len());
                for p in &path {
                    let p = p.convert_to_i32();
                    points.extend(stamp.iter().map(|&offset| (p + offset).convert_to_u32()));
                }
                points
            }
            LinePositioning::BetweenPoints { .. } => path,
        };

        Ok(vec![MaskPoints { points, color }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashed_horizontal_line_skips_gaps() {
        let style = LineStyle::Dashed {
            color: colors::WHITE,
            thickness: 0,
        };
        let pos = LinePositioning::Horizontal {
            start: Point::new(0, 0),
            length: 19,
        };
        let line: Line<i32> = Line::new(pos, style);
        let mask = line.get_mask().unwrap();
        let xs: Vec<u32> = mask[0].points.iter().map(|p| p.x).collect();
        assert_eq!(xs, vec![0, 1, 2, 3, 4, 5, 10, 11, 12, 13, 14, 15]);
    }

    #[test]
    fn dashed_diagonal_line_skips_gaps() {
        let style = LineStyle::Dashed {
            color: colors::WHITE,
            thickness: 0,
        };
        let pos = LinePositioning::BetweenPoints {
            start: Point::new(0, 0),
            end: Point::new(9, 9),
        };
        let line: Line<i32> = Line::new(pos, style);
        let mask = line.get_mask().unwrap();
        assert_eq!(mask[0].points.len(), 6);
        assert_eq!(mask[0].points[5], Point::new(5, 5));
    }

    #[test]
    fn dashed_thick_line_applies_thickness() {
        let style = LineStyle::Dashed {
            color: colors::WHITE,
            thickness: 1,
        };
        let pos = LinePositioning::Vertical {
            start: Point::new(5, 0),
            length: 9,
        };
        let line: Line<i32> = Line::new(pos, style);
        let mask = line.get_mask().unwrap();
        // 6 dashed pixels, drawn 3 pixels wide
        assert_eq!(mask[0].points.len(), 18);
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
        assert!(
            !points.is_empty(),
            "Right-to-left line should produce points"
        );
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
        assert!(
            points.len() <= 1,
            "Single-point line should produce 0 or 1 points"
        );
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

    #[test]
    fn diagonal_line_thickness_widens_line() {
        let mask_for = |thickness| {
            let pos = LinePositioning::BetweenPoints {
                start: Point::new(10, 10),
                end: Point::new(30, 20),
            };
            let style = LineStyle::Solid {
                color: colors::WHITE,
                thickness,
            };
            let mut points = Line::<i32>::new(pos, style).get_mask().unwrap()[0]
                .points
                .clone();
            points.sort_by_key(|p| (p.x, p.y));
            points.dedup();
            points
        };
        let thin = mask_for(0);
        let thick = mask_for(2);
        assert!(
            thick.len() > 3 * thin.len(),
            "{} vs {}",
            thick.len(),
            thin.len()
        );
        // every pixel of the thin line is covered by the thick one
        assert!(thin.iter().all(|p| thick.contains(p)));
        // and the thick line stays within `thickness` of the thin line
        assert!(thick.iter().all(|p| thin.iter().any(|q| {
            let (dx, dy) = (p.x as i64 - q.x as i64, p.y as i64 - q.y as i64);
            dx * dx + dy * dy <= 4
        })));
    }

    #[test]
    fn disc_offsets_radius_one_is_a_plus_shape() {
        assert_eq!(disc_offsets(1).len(), 5);
        assert_eq!(disc_offsets(0), vec![Point::new(0, 0)]);
    }
}
