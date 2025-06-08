use std::{
    fmt::Debug,
    ops::{Add, Div, Mul, Sub},
};

use rgb::RGB8;

use crate::common::Result;

use super::point::Point;

pub trait Graphable:
    PartialOrd
    + Into<f64>
    + PartialEq
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Clone
    + Copy
    + Sized
    + Debug
{
}

impl<T> Graphable for T where
    T: PartialOrd
        + Into<f64>
        + PartialEq
        + Add<Output = Self>
        + Sub<Output = Self>
        + Mul<Output = Self>
        + Div<Output = Self>
        + Clone
        + Copy
        + Sized
        + Debug
{
}

pub enum DrawPositioning {
    CenteredAt(Point<u32>),
    Between(Point<u32>, Point<u32>),
}

pub struct MaskPoints {
    pub points: Vec<Point<u32>>,
    pub color: RGB8,
}

pub trait Drawable {
    fn bounding_width(&self) -> u32;
    fn bounding_height(&self) -> u32;
    fn get_mask(&self, pos: DrawPositioning) -> Result<Vec<MaskPoints>>;
}
