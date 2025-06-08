use super::{
    colors,
    common::{Drawable, MaskPoints},
    limits::Limits,
    point::Point,
};
use crate::common::Result;
use rgb::RGB8;

#[derive(Debug, Clone)]
pub enum MarkerStyle {
    FilledSquare { size: u32, color: RGB8 },
    HollowSquare { size: u32, color: RGB8 },
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
            MarkerStyle::FilledSquare { size, color: _ } => *size,
            MarkerStyle::HollowSquare { size, color: _ } => *size,
        }
    }
}

impl Marker {
    pub fn new(center: Point<u32>, style: MarkerStyle) -> Marker {
        Marker { center, style }
    }

    pub fn limits(&self) -> Limits<u32> {
        let size = self.style.size();
        Limits::new(self.center - size, self.center + size)
    }

    pub fn style(&self) -> &MarkerStyle {
        &self.style
    }

    pub fn center(&self) -> &Point<u32> {
        &self.center
    }
}

impl Drawable for Marker {
    fn bounding_width(&self) -> u32 {
        self.style.size()
    }

    fn bounding_height(&self) -> u32 {
        self.style.size()
    }

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
                let top = Point::<u32>::range(
                    &Point::new(self.center.x - size, self.center.y + size),
                    &Point::new(self.center.x + size, self.center.y + size),
                );
                let bottom = Point::<u32>::range(
                    &Point::new(self.center.x - size, self.center.y - size),
                    &Point::new(self.center.x + size, self.center.y - size),
                );
                let right = Point::<u32>::range(
                    &Point::new(self.center.x + size, self.center.y - size),
                    &Point::new(self.center.x + size, self.center.y + size),
                );
                let left = Point::<u32>::range(
                    &Point::new(self.center.x - size, self.center.y - size),
                    &Point::new(self.center.x - size, self.center.y + size),
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
        };
        Ok(mask_points)
    }
}
