use core::f32;

use super::{
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

#[derive(Debug, Clone)]
pub enum MarkerStyle {
    FilledSquare { size: u32, color: RGB8 },
    HollowSquare { size: u32, color: RGB8 },
    FilledCircle { size: u32, color: RGB8 },
    HollowCircle { size: u32, color: RGB8 },
}

#[derive(Debug)]
pub struct Marker {
    style: MarkerStyle,
    center: Point<u32>,
}

impl MarkerStyle {
    pub fn default() -> MarkerStyle {
        MarkerStyle::FilledSquare {
            size: 0,
            color: colors::WHITE,
        }
    }

    pub fn size(&self) -> u32 {
        match self {
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
        let max = self.center + size;
        Limits::new(min, max)
    }

    pub fn style(&self) -> &MarkerStyle {
        &self.style
    }

    pub fn center(&self) -> &Point<u32> {
        &self.center
    }
}

impl Drawable for Marker {
    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let mask_points = match self.style {
            MarkerStyle::FilledSquare { color, size: _ } => {
                let limits = self.limits();
                let points = Point::<u32>::limit_range(limits);
                vec![MaskPoints {
                    points,
                    color: color.clone(),
                }]
            }
            MarkerStyle::HollowSquare { color, size } => {
                let x_lo = self.center.x.saturating_sub(size);
                let y_lo = self.center.y.saturating_sub(size);
                let x_hi = self.center.x + size;
                let y_hi = self.center.y + size;
                let top = Point::<u32>::range(
                    &Point::new(x_lo, y_hi),
                    &Point::new(x_hi, y_hi),
                );
                let bottom = Point::<u32>::range(
                    &Point::new(x_lo, y_lo),
                    &Point::new(x_hi, y_lo),
                );
                let right = Point::<u32>::range(
                    &Point::new(x_hi, y_lo),
                    &Point::new(x_hi, y_hi),
                );
                let left = Point::<u32>::range(
                    &Point::new(x_lo, y_lo),
                    &Point::new(x_lo, y_hi),
                );
                vec![
                    MaskPoints {
                        points: top,
                        color: color.clone(),
                    },
                    MaskPoints {
                        points: bottom,
                        color: color.clone(),
                    },
                    MaskPoints {
                        points: right,
                        color: color.clone(),
                    },
                    MaskPoints {
                        points: left,
                        color: color.clone(),
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
