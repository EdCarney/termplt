use core::f32;

use super::{
    canvas::Canvas,
    colors,
    common::{Drawable, MaskPoints},
    limits::Limits,
    point::Point,
};
use crate::{
    common::Result,
    plotting::common::{IntConvertable, UIntConvertable},
};
use rgb::RGB8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarkerStyle {
    /// No marker is drawn (e.g. for a line-only series).
    None,
    FilledSquare {
        size: u32,
        color: RGB8,
    },
    HollowSquare {
        size: u32,
        color: RGB8,
    },
    FilledCircle {
        size: u32,
        color: RGB8,
    },
    HollowCircle {
        size: u32,
        color: RGB8,
    },
}

#[derive(Debug)]
pub struct Marker {
    style: MarkerStyle,
    center: Point<u32>,
}

impl Default for MarkerStyle {
    fn default() -> MarkerStyle {
        MarkerStyle::FilledSquare {
            size: 0,
            color: colors::WHITE,
        }
    }
}

impl MarkerStyle {
    /// The marker color; `None` for [`MarkerStyle::None`].
    pub fn color(&self) -> Option<RGB8> {
        match self {
            MarkerStyle::None => None,
            MarkerStyle::FilledSquare { color, .. }
            | MarkerStyle::HollowSquare { color, .. }
            | MarkerStyle::FilledCircle { color, .. }
            | MarkerStyle::HollowCircle { color, .. } => Some(*color),
        }
    }

    /// Marker radius in pixels; zero for [`MarkerStyle::None`].
    pub fn size(&self) -> u32 {
        match self {
            MarkerStyle::None => 0,
            MarkerStyle::FilledSquare { size, .. }
            | MarkerStyle::HollowSquare { size, .. }
            | MarkerStyle::FilledCircle { size, .. }
            | MarkerStyle::HollowCircle { size, .. } => *size,
        }
    }
}

impl Marker {
    pub fn new(center: Point<u32>, style: MarkerStyle) -> Marker {
        Marker { center, style }
    }

    pub fn limits(&self) -> Limits<u32> {
        let size = self.style.size();
        let min = Point::new(
            self.center.x.saturating_sub(size),
            self.center.y.saturating_sub(size),
        );
        let max = Point::new(
            self.center.x.saturating_add(size),
            self.center.y.saturating_add(size),
        );
        Limits::new(min, max)
    }

    pub fn style(&self) -> &MarkerStyle {
        &self.style
    }

    pub fn center(&self) -> &Point<u32> {
        &self.center
    }
}

/// The pixel offsets from a marker's center that a marker of this style covers, sorted and
/// without duplicates. Every marker of a style covers the same offsets (clamped at 0 near the
/// canvas edge), so they are computed once and stamped at each point.
pub(crate) fn marker_stamp(style: &MarkerStyle) -> Result<Vec<Point<i32>>> {
    // far enough from the origin that no offset is clamped
    let c = style.size().saturating_add(1);
    let mut offsets: Vec<Point<i32>> = Marker::new(Point::new(c, c), *style)
        .get_mask()?
        .iter()
        .flat_map(|mask| mask.points.iter())
        .map(|p| Point::new(p.x as i32 - c as i32, p.y as i32 - c as i32))
        .collect();
    offsets.sort_by_key(|p| (p.x, p.y));
    offsets.dedup();
    Ok(offsets)
}

/// Draws a marker stamp (from [`marker_stamp`]) centered on `center`; offsets that would fall
/// below 0 are clamped to 0, like [`Marker::get_mask`].
pub(crate) fn draw_marker(
    canvas: &mut Canvas,
    center: Point<u32>,
    stamp: &[Point<i32>],
    color: RGB8,
) {
    let clamp = |v: i64| v.clamp(0, u32::MAX as i64) as u32;
    for offset in stamp {
        let x = clamp(center.x as i64 + offset.x as i64);
        let y = clamp(center.y as i64 + offset.y as i64);
        canvas.put(x, y, color);
    }
}

impl Drawable for Marker {
    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let mask_points = match self.style {
            MarkerStyle::None => Vec::new(),
            MarkerStyle::FilledSquare { color, size: _ } => {
                let limits = self.limits();
                let points = Point::<u32>::limit_range(limits);
                vec![MaskPoints { points, color }]
            }
            MarkerStyle::HollowSquare { color, size } => {
                let x_lo = self.center.x.saturating_sub(size);
                let y_lo = self.center.y.saturating_sub(size);
                let x_hi = self.center.x.saturating_add(size);
                let y_hi = self.center.y.saturating_add(size);
                let top = Point::<u32>::range(&Point::new(x_lo, y_hi), &Point::new(x_hi, y_hi));
                let bottom = Point::<u32>::range(&Point::new(x_lo, y_lo), &Point::new(x_hi, y_lo));
                let right = Point::<u32>::range(&Point::new(x_hi, y_lo), &Point::new(x_hi, y_hi));
                let left = Point::<u32>::range(&Point::new(x_lo, y_lo), &Point::new(x_lo, y_hi));
                vec![
                    MaskPoints { points: top, color },
                    MaskPoints {
                        points: bottom,
                        color,
                    },
                    MaskPoints {
                        points: right,
                        color,
                    },
                    MaskPoints {
                        points: left,
                        color,
                    },
                ]
            }
            MarkerStyle::FilledCircle { size, color } => {
                let get_point_fn = |x_adj: f32, y_adj: f32| -> Point<u32> {
                    let size = size as i32;
                    let x_adj = if x_adj > 0. {
                        i32::min(x_adj.round().convert_to_i32(), size)
                    } else {
                        i32::max(x_adj.round().convert_to_i32(), -size)
                    };
                    let y_adj = if y_adj > 0. {
                        i32::min(y_adj.round().convert_to_i32(), size)
                    } else {
                        i32::max(y_adj.round().convert_to_i32(), -size)
                    };

                    let x = (self.center.x.convert_to_i32() + x_adj).convert_to_u32();
                    let y = (self.center.y.convert_to_i32() + y_adj).convert_to_u32();

                    Point::new(x, y)
                };

                let mut points = Vec::new();
                let radius = size as f32;
                let step = f32::atan(1.0 / radius);
                let mut angle = 0.;
                while angle < f32::consts::FRAC_PI_2 + step {
                    let x_adj = radius * f32::cos(angle);
                    let y_adj = radius * f32::sin(angle);

                    let iter_points = Point::<u32>::range(
                        &get_point_fn(-x_adj, -y_adj),
                        &get_point_fn(x_adj, y_adj),
                    );

                    points.extend(iter_points);
                    angle += step;
                }
                vec![MaskPoints { points, color }]
            }
            MarkerStyle::HollowCircle { size, color } => {
                let get_point_fn = |x_adj: f32, y_adj: f32| -> Point<u32> {
                    let size = size as i32;
                    let x_adj = if x_adj > 0. {
                        i32::min(x_adj.round().convert_to_i32(), size)
                    } else {
                        i32::max(x_adj.round().convert_to_i32(), -size)
                    };
                    let y_adj = if y_adj > 0. {
                        i32::min(y_adj.round().convert_to_i32(), size)
                    } else {
                        i32::max(y_adj.round().convert_to_i32(), -size)
                    };

                    let x = (self.center.x.convert_to_i32() + x_adj).convert_to_u32();
                    let y = (self.center.y.convert_to_i32() + y_adj).convert_to_u32();

                    Point::new(x, y)
                };

                let mut points = Vec::new();
                let radius = size as f32;
                let step = f32::atan(1.0 / radius);
                let mut angle = 0.;
                while angle < f32::consts::FRAC_PI_2 + step {
                    let x_adj = radius * f32::cos(angle);
                    let y_adj = radius * f32::sin(angle);

                    let iter_points = vec![
                        get_point_fn(x_adj, y_adj),
                        get_point_fn(-x_adj, y_adj),
                        get_point_fn(x_adj, -y_adj),
                        get_point_fn(-x_adj, -y_adj),
                    ];

                    points.extend(iter_points);
                    angle += step;
                }
                vec![MaskPoints { points, color }]
            }
        };
        Ok(mask_points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_marker_draws_nothing() {
        let marker = Marker::new(Point::new(10, 10), MarkerStyle::None);
        assert_eq!(MarkerStyle::None.size(), 0);
        assert!(marker.get_mask().unwrap().is_empty());
    }

    #[test]
    fn filled_square_covers_its_limits() {
        let style = MarkerStyle::FilledSquare {
            size: 2,
            color: colors::RED,
        };
        let mask = Marker::new(Point::new(10, 10), style).get_mask().unwrap();
        assert_eq!(mask[0].points.len(), 25);
    }
}
