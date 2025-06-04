use super::common::Graphable;
use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Point<T: Graphable<T>> {
    pub x: T,
    pub y: T,
}

impl<T: Graphable<T>> Point<T> {
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

impl<T: Graphable<T>> Point<T> {
    pub fn new(x: T, y: T) -> Point<T> {
        Point { x, y }
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
}
