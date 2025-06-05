use super::{common::Graphable, limits::Limits};
use std::ops::{Add, Div, Mul, Sub};

pub trait PointCollection<T: Graphable<T>> {
    fn limits(&self) -> Option<Limits<T>>;
}

impl<T: Graphable<T>> PointCollection<T> for Vec<Point<T>> {
    fn limits(&self) -> Option<Limits<T>> {
        self.as_slice().limits()
    }
}

impl<T: Graphable<T>> PointCollection<T> for &[Point<T>] {
    fn limits(&self) -> Option<Limits<T>> {
        // limits must have at least two points
        if self.iter().len() < 2 {
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
pub struct Point<T: Graphable<T>> {
    pub x: T,
    pub y: T,
}

impl<T: Graphable<T>> Point<T> {
    pub fn new(x: T, y: T) -> Point<T> {
        Point { x, y }
    }

    pub fn try_into_point<U>(self) -> Result<Point<U>, U::Error>
    where
        U: Graphable<U> + TryFrom<T>,
    {
        Ok(Point {
            x: self.x.try_into()?,
            y: self.y.try_into()?,
        })
    }
}

impl<T: Graphable<T>> Add for Point<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Point {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl<T: Graphable<T>> Sub for Point<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Point {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl<T: Graphable<T>> Add<T> for Point<T> {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        Point {
            x: self.x + rhs,
            y: self.y + rhs,
        }
    }
}

impl<T: Graphable<T>> Sub<T> for Point<T> {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        Point {
            x: self.x - rhs,
            y: self.y - rhs,
        }
    }
}

impl<T: Graphable<T>> Mul<T> for Point<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Point {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl<T: Graphable<T>> Div<T> for Point<T> {
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
        let p = vec![Point { x: 10, y: 20 }];
        assert_eq!(p.limits(), None);
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
