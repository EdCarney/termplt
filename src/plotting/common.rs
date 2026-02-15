use super::{limits::Limits, point::Point};
use crate::common::Result;
use rgb::RGB8;
use std::{
    fmt::Debug,
    ops::{Add, Div, Mul, Sub},
};

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

#[derive(Debug)]
pub struct MaskPoints {
    pub points: Vec<Point<u32>>,
    pub color: RGB8,
}

pub trait Drawable {
    fn get_mask(&self) -> Result<Vec<MaskPoints>>;
}

pub trait Convertable<U> {
    type ConvertTo;
    fn convert_to(&self, convert_fn: fn(f64) -> U) -> Self::ConvertTo;
}

impl<T: Graphable, U: Graphable> Convertable<U> for T {
    type ConvertTo = U;
    fn convert_to(&self, convert_fn: fn(f64) -> U) -> Self::ConvertTo {
        let value: f64 = self.clone().into();
        convert_fn(value)
    }
}

pub trait UIntConvertable: Convertable<u32> {
    fn convert_to_u32(&self) -> Self::ConvertTo;
}

fn safe_f64_to_u32(v: f64) -> u32 {
    v.clamp(0.0, u32::MAX as f64) as u32
}

impl<T: Convertable<u32>> UIntConvertable for T {
    fn convert_to_u32(&self) -> Self::ConvertTo {
        self.convert_to(safe_f64_to_u32)
    }
}

pub trait IntConvertable: Convertable<i32> {
    fn convert_to_i32(&self) -> Self::ConvertTo;
}

fn safe_f64_to_i32(v: f64) -> i32 {
    v.clamp(i32::MIN as f64, i32::MAX as f64) as i32
}

impl<T: Convertable<i32>> IntConvertable for T {
    fn convert_to_i32(&self) -> Self::ConvertTo {
        self.convert_to(safe_f64_to_i32)
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

pub trait Scalable<T, U>
where
    T: FloatConvertable + Graphable,
    U: FloatConvertable + Graphable,
{
    type ScaleTo;
    fn scale_to(self, old_limits: &Limits<T>, new_limits: &Limits<U>) -> Self::ScaleTo;
}

pub trait Shiftable<T>
where
    T: FloatConvertable + Graphable,
{
    fn shift_by(self, amount: Point<T>) -> Self;
}
