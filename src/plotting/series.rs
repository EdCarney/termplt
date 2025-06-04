use super::{
    markers::{LineStyle, MarkerStyle},
    point::Point,
};
use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, PartialEq)]
pub struct Series {
    data: Vec<Point>,
    marker_style: MarkerStyle,
    line_style: LineStyle,
}

impl Series {
    pub fn new(data: &[Point]) -> Series {
        Series {
            data: Vec::from(data),
            marker_style: MarkerStyle::default(),
            line_style: LineStyle::default(),
        }
    }
}

impl Add<f32> for Series {
    type Output = Self;

    fn add(self, rhs: f32) -> Self::Output {
        let data: Vec<Point> = self.data.into_iter().map(|p| p + rhs).collect();
        Series::new(&data)
    }
}

impl Sub<f32> for Series {
    type Output = Self;

    fn sub(self, rhs: f32) -> Self::Output {
        let data: Vec<Point> = self.data.into_iter().map(|p| p - rhs).collect();
        Series::new(&data)
    }
}

impl Mul<f32> for Series {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        let data: Vec<Point> = self.data.into_iter().map(|p| p * rhs).collect();
        Series::new(&data)
    }
}

impl Div<f32> for Series {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        let data: Vec<Point> = self.data.into_iter().map(|p| p / rhs).collect();
        Series::new(&data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_f32_to_series() {
        let p1 = Point { x: 10.0, y: 20.0 };
        let p2 = Point { x: 12.5, y: 17.5 };
        let p3 = Point { x: 15.0, y: 15.0 };
        let data: Vec<Point> = vec![p1, p2, p3];
        let s1 = Series::new(&data);
        let x = 2.0;

        let s2 = s1 + x;
        assert_eq!(s2.data[0], Point { x: 12.0, y: 22.0 });
        assert_eq!(s2.data[1], Point { x: 14.5, y: 19.5 });
        assert_eq!(s2.data[2], Point { x: 17.0, y: 17.0 });
    }

    #[test]
    fn subtract_f32_from_series() {
        let p1 = Point { x: 10.0, y: 20.0 };
        let p2 = Point { x: 12.5, y: 17.5 };
        let p3 = Point { x: 15.0, y: 15.0 };
        let data: Vec<Point> = vec![p1, p2, p3];
        let s1 = Series::new(&data);
        let x = 2.0;

        let s2 = s1 - x;
        assert_eq!(s2.data[0], Point { x: 8.0, y: 18.0 });
        assert_eq!(s2.data[1], Point { x: 10.5, y: 15.5 });
        assert_eq!(s2.data[2], Point { x: 13.0, y: 13.0 });
    }

    #[test]
    fn multiply_series_by_f32() {
        let p1 = Point { x: 10.0, y: 20.0 };
        let p2 = Point { x: 12.5, y: 17.5 };
        let p3 = Point { x: 15.0, y: 15.0 };
        let data: Vec<Point> = vec![p1, p2, p3];
        let s1 = Series::new(&data);
        let x = 2.0;

        let s2 = s1 * x;
        assert_eq!(s2.data[0], Point { x: 20.0, y: 40.0 });
        assert_eq!(s2.data[1], Point { x: 25.0, y: 35.0 });
        assert_eq!(s2.data[2], Point { x: 30.0, y: 30.0 });
    }

    #[test]
    fn divide_series_by_f32() {
        let p1 = Point { x: 10.0, y: 20.0 };
        let p2 = Point { x: 12.5, y: 17.5 };
        let p3 = Point { x: 15.0, y: 15.0 };
        let data: Vec<Point> = vec![p1, p2, p3];
        let s1 = Series::new(&data);
        let x = 2.0;

        let s2 = s1 / x;
        assert_eq!(s2.data[0], Point { x: 5.0, y: 10.0 });
        assert_eq!(s2.data[1], Point { x: 6.25, y: 8.75 });
        assert_eq!(s2.data[2], Point { x: 7.5, y: 7.5 });
    }
}
