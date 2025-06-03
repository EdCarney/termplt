use super::point::Point;
use std::ops::{Add, Div, Mul, Sub};

pub struct Series {
    data: Vec<Point>,
}

impl Add<f32> for Series {
    type Output = Self;

    fn add(self, rhs: f32) -> Self::Output {
        let data = self.data.into_iter().map(|p| p + rhs).collect();
        Series { data }
    }
}

impl Sub<f32> for Series {
    type Output = Self;

    fn sub(self, rhs: f32) -> Self::Output {
        let data = self.data.into_iter().map(|p| p - rhs).collect();
        Series { data }
    }
}

impl Mul<f32> for Series {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        let data = self.data.into_iter().map(|p| p * rhs).collect();
        Series { data }
    }
}

impl Div<f32> for Series {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        let data = self.data.into_iter().map(|p| p / rhs).collect();
        Series { data }
    }
}
