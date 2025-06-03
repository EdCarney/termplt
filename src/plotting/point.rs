use std::ops::{Add, Div, Mul, Sub};

pub struct Point {
    x: f32,
    y: f32,
}

impl Add for Point {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Point {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Point {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Point {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Add<f32> for Point {
    type Output = Self;

    fn add(self, rhs: f32) -> Self::Output {
        Point {
            x: self.x + rhs,
            y: self.y + rhs,
        }
    }
}

impl Sub<f32> for Point {
    type Output = Self;

    fn sub(self, rhs: f32) -> Self::Output {
        Point {
            x: self.x - rhs,
            y: self.y - rhs,
        }
    }
}

impl Mul<f32> for Point {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Point {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Div<f32> for Point {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
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
    fn subtract_point_to_point() {
        let p1 = Point { x: 10.0, y: 15.5 };
        let p2 = Point { x: 5.5, y: 7.5 };

        let p3 = p1 - p2;
        assert_eq!(p3.x, 4.5);
        assert_eq!(p3.y, 8.0);
    }
}
