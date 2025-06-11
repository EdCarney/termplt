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

pub struct MaskPoints {
    pub points: Vec<Point<u32>>,
    pub color: RGB8,
}

pub trait Drawable {
    fn bounding_width(&self) -> u32;
    fn bounding_height(&self) -> u32;
    fn get_mask(&self) -> Result<Vec<MaskPoints>>;
}

pub trait Convertable<U> {
    type ConvertTo;
    fn convert_to(&self, convert_fn: unsafe fn(f64) -> U) -> Self::ConvertTo;
}

impl<T: Graphable, U: Graphable> Convertable<U> for T {
    type ConvertTo = U;
    fn convert_to(&self, convert_fn: unsafe fn(f64) -> U) -> Self::ConvertTo {
        let value: f64 = self.clone().into();
        unsafe { convert_fn(value) }
    }
}

pub trait IntConvertable: Convertable<u32> {
    fn convert_to_u32(&self) -> Self::ConvertTo;
}

impl<T: Convertable<u32>> IntConvertable for T {
    fn convert_to_u32(&self) -> Self::ConvertTo {
        self.convert_to(f64::to_int_unchecked)
    }
}

pub trait FloatConvertable: Convertable<f64> {
    fn convert_to_f64(&self) -> Self::ConvertTo;
}

impl<T: Convertable<f64>> FloatConvertable for T {
    fn convert_to_f64(&self) -> Self::ConvertTo {
        self.convert_to(f64::from)
    }
}
