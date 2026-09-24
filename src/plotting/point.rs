use super::{
    common::{Convertable, FloatConvertable, Graphable, UIntConvertable},
    limits::Limits,
};
use std::ops::{Add, Div, Mul, Sub};

/// A 2D point.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Point<T: Graphable> {
    /// The x coordinate.
    pub x: T,
    /// The y coordinate.
    pub y: T,
}

impl<T: Graphable, U: Graphable> Convertable<U> for Point<T> {
    type ConvertTo = Point<U>;
    fn convert_to(&self, convert_fn: fn(f64) -> U) -> Self::ConvertTo {
        let x = self.x.convert_to(convert_fn);
        let y = self.y.convert_to(convert_fn);
        Point { x, y }
    }
}

impl<T> Point<T>
where
    T: Graphable + UIntConvertable,
{
    /// Generates the set of points between the start and end (inclusive).
    pub fn range(from: &Point<T>, to: &Point<T>) -> Vec<Point<u32>> {
        let from = from.convert_to_u32();
        let to = to.convert_to_u32();
        (from.x..=to.x)
            .flat_map(|x| (from.y..=to.y).map(move |y| Point::new(x, y)))
            .collect()
    }

    /// Generates the set of points between the limits min/max (inclusive).
    pub fn limit_range(limits: Limits<T>) -> Vec<Point<u32>> {
        Point::<T>::range(limits.min(), limits.max())
    }

    /// Rounds point to the nearest integer values and converts to u32.
    pub fn round(&self) -> Point<u32> {
        let x = self.x.convert_to_f64().round().convert_to_u32();
        let y = self.y.convert_to_f64().round().convert_to_u32();
        Point { x, y }
    }

    /// Rounds point to the largest integer values greater than or equal to the current values and
    /// converts to u32.
    pub fn ceil(&self) -> Point<u32> {
        let x = self.x.convert_to_f64().ceil().convert_to_u32();
        let y = self.y.convert_to_f64().ceil().convert_to_u32();
        Point { x, y }
    }

    /// Rounds point to the largest integer values less than or equal to the current values and
    /// converts to u32.
    pub fn floor(&self) -> Point<u32> {
        let x = self.x.convert_to_f64().floor().convert_to_u32();
        let y = self.y.convert_to_f64().floor().convert_to_u32();
        Point { x, y }
    }
}

impl<T> Point<T>
where
    T: FloatConvertable + Graphable,
{
    /// Creates a point.
    pub fn new(x: T, y: T) -> Point<T> {
        Point { x, y }
    }

    /// The Euclidean distance to `other`.
    pub fn dist<U>(&self, other: &Point<U>) -> f64
    where
        U: FloatConvertable + Graphable,
    {
        let diff = self.convert_to_f64() - other.convert_to_f64();
        f64::sqrt(diff.x.powi(2) + diff.y.powi(2))
    }
}

impl<T: Graphable> Add for Point<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Point {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<T: Graphable> Sub for Point<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Point {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<T: Graphable> Add<T> for Point<T> {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        Point {
            x: self.x + rhs,
            y: self.y + rhs,
        }
    }
}

impl<T: Graphable> Sub<T> for Point<T> {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        Point {
            x: self.x - rhs,
            y: self.y - rhs,
        }
    }
}

impl<T: Graphable> Mul<T> for Point<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Point {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl<T: Graphable> Div<T> for Point<T> {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        Point {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_point_to_point() {
        let p1 = Point { x: 10.0, y: 15.5 };
        let p2 = Point { x: 5.5, y: 7.5 };

        let p3 = p1 + p2;
        assert_eq!(p3.x, 15.5);
        assert_eq!(p3.y, 23.0);
    }

    #[test]
    fn subtract_point_from_point() {
        let p1 = Point { x: 10.0, y: 15.5 };
        let p2 = Point { x: 5.5, y: 7.5 };

        let p3 = p1 - p2;
        assert_eq!(p3.x, 4.5);
        assert_eq!(p3.y, 8.0);
    }

    #[test]
    fn add_f32_to_point() {
        let p1 = Point { x: 10.0, y: 15.5 };
        let x = 15.5;

        let p2 = p1 + x;
        assert_eq!(p2.x, 25.5);
        assert_eq!(p2.y, 31.0);
    }

    #[test]
    fn subtract_f32_from_point() {
        let p1 = Point { x: 10.0, y: 15.5 };
        let x = 15.5;

        let p2 = p1 - x;
        assert_eq!(p2.x, -5.5);
        assert_eq!(p2.y, 0.0);
    }

    #[test]
    fn multiply_point_by_f32() {
        let p1 = Point { x: 10.0, y: 15.5 };
        let x = 3.0;

        let p2 = p1 * x;
        assert_eq!(p2.x, 30.0);
        assert_eq!(p2.y, 46.5);
    }

    #[test]
    fn divide_point_by_f32() {
        let p1 = Point { x: 10.0, y: 15.0 };
        let x = 5.0;

        let p2 = p1 / x;
        assert_eq!(p2.x, 2.0);
        assert_eq!(p2.y, 3.0);
    }
}
