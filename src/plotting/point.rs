use super::{
    common::{Convertable, Graphable, UIntConvertable},
    limits::Limits,
};
use std::ops::{Add, Div, Mul, Sub};

pub trait PointCollection<T: Graphable> {
    fn limits(&self) -> Option<Limits<T>>;
}

impl<T: Graphable> PointCollection<T> for Vec<Point<T>> {
    fn limits(&self) -> Option<Limits<T>> {
        self.as_slice().limits()
    }
}

impl<T: Graphable> PointCollection<T> for &[Point<T>] {
    fn limits(&self) -> Option<Limits<T>> {
        // limits must have at least one point
        if self.iter().len() == 0 {
            return None;
        }
        let first = self.first().unwrap();
        let min_x = self
            .iter()
            .fold(first.x, |min, val| if val.x < min { val.x } else { min });
        let min_y = self
            .iter()
            .fold(first.y, |min, val| if val.y < min { val.y } else { min });
        let max_x = self
            .iter()
            .fold(first.x, |max, val| if val.x > max { val.x } else { max });
        let max_y = self
            .iter()
            .fold(first.y, |max, val| if val.y > max { val.y } else { max });
        let min = Point { x: min_x, y: min_y };
        let max = Point { x: max_x, y: max_y };
        Some(Limits::new(min, max))
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Point<T: Graphable> {
    pub x: T,
    pub y: T,
}

impl<T: Graphable, U: Graphable> Convertable<U> for Point<T> {
    type ConvertTo = Point<U>;
    fn convert_to(&self, convert_fn: unsafe fn(f64) -> U) -> Self::ConvertTo {
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
}

impl<T: Graphable> Point<T> {
    pub fn new(x: T, y: T) -> Point<T> {
        Point { x, y }
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

    #[test]
    fn point_collection_limits_empty() {
        let p: Vec<Point<u32>> = vec![];
        assert_eq!(p.limits(), None);
    }

    #[test]
    fn point_collection_limits_single() {
        let limits = vec![Point { x: 10, y: 20 }].limits();
        assert!(limits.is_some());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(10, 20), Point::new(10, 20))
        );
    }

    #[test]
    fn point_collection_limits_multiple_1() {
        let p1 = Point { x: 0, y: 0 };
        let p2 = Point { x: 10, y: 20 };
        let limits = vec![p1, p2].limits();

        assert!(limits.is_some());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(0, 0), Point::new(10, 20))
        );
    }

    #[test]
    fn point_collection_limits_multiple_2() {
        let p1 = Point { x: -5, y: 50 };
        let p2 = Point { x: 10, y: -20 };
        let p3 = Point { x: 100, y: 20 };
        let limits = vec![p1, p2, p3].limits();

        assert!(limits.is_some());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(-5, -20), Point::new(100, 50))
        );
    }
}
