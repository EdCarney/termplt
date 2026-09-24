use super::point::Point;
use crate::common::Result;
use rgb::RGB8;
use std::{
    fmt::Debug,
    ops::{Add, Div, Mul, Sub},
};

/// Lossy conversion to `f64` (`as` semantics), for every primitive numeric type. Unlike
/// `Into<f64>`, it covers `i64`, `u64`, `usize` and friends; values beyond 2^53 lose precision,
/// which is invisible at plot resolution.
pub trait ToF64: Copy {
    /// Converts the value to `f64`.
    fn to_f64(self) -> f64;
}

macro_rules! impl_to_f64 {
    ($($t:ty),*) => {
        $(impl ToF64 for $t {
            #[inline]
            fn to_f64(self) -> f64 {
                self as f64
            }
        })*
    };
}

impl_to_f64!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
);

/// Numeric types that can be plotted: every primitive integer and float type.
pub trait Graphable:
    PartialOrd
    + ToF64
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
        + ToF64
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

/// Pixels (in canvas coordinates, with (0, 0) at the lower left) to paint in one color.
#[derive(Debug)]
pub struct MaskPoints {
    /// The pixels.
    pub points: Vec<Point<u32>>,
    /// Their color.
    pub color: RGB8,
}

/// Something that can be rendered to pixels.
pub trait Drawable {
    /// The pixels to paint, in drawing order.
    fn get_mask(&self) -> Result<Vec<MaskPoints>>;
}

pub trait Convertable<U> {
    type ConvertTo;
    fn convert_to(&self, convert_fn: fn(f64) -> U) -> Self::ConvertTo;
}

impl<T: Graphable, U: Graphable> Convertable<U> for T {
    type ConvertTo = U;
    fn convert_to(&self, convert_fn: fn(f64) -> U) -> Self::ConvertTo {
        let value = self.to_f64();
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
