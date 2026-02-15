use super::{
    common::{
        Convertable, Drawable, FloatConvertable, Graphable, MaskPoints, Scalable, Shiftable,
        UIntConvertable,
    },
    limits::Limits,
    line::{Line, LineStyle},
    line_positioning::LinePositioning,
    marker::{Marker, MarkerStyle},
    point::Point,
};
use crate::common::Result;
use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, Clone)]
pub struct Series<T: Graphable> {
    data: Vec<Point<T>>,
    marker_style: MarkerStyle,
    line_style: Option<LineStyle>,
}

impl<T: Graphable, U: Graphable> Convertable<U> for Series<T> {
    type ConvertTo = Series<U>;
    fn convert_to(&self, convert_fn: fn(f64) -> U) -> Self::ConvertTo {
        let data = self
            .data
            .iter()
            .map(|&p| p.convert_to(convert_fn))
            .collect::<Vec<_>>();
        self.clone_with(&data)
    }
}

impl<T: Graphable> Series<T> {
    pub fn new(data: &[Point<T>]) -> Series<T> {
        if data.is_empty() {
            panic!("Data series cannot be empty");
        }

        Series {
            data: Vec::from(data),
            marker_style: MarkerStyle::default(),
            line_style: None,
        }
    }

    pub fn clone_with<U: Graphable>(&self, data: &[Point<U>]) -> Series<U> {
        let marker_style = self.marker_style.clone();
        let line_style = self.line_style.clone();
        Series {
            data: Vec::from(data),
            marker_style,
            line_style,
        }
    }

    pub fn data(&self) -> &[Point<T>] {
        &self.data
    }

    pub fn marker_style(&self) -> &MarkerStyle {
        &self.marker_style
    }

    pub fn line_style(&self) -> &Option<LineStyle> {
        &self.line_style
    }

    pub fn with_marker_style(mut self, marker_style: MarkerStyle) -> Self {
        self.marker_style = marker_style;
        self
    }

    pub fn with_line_style(mut self, line_style: LineStyle) -> Self {
        self.line_style = Some(line_style);
        self
    }
}

impl<T: UIntConvertable + Graphable> Drawable for Series<T> {
    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let mut mask_points = self
            .data()
            .iter()
            .flat_map(|&p| {
                Marker::new(p.convert_to_u32(), self.marker_style.clone())
                    .get_mask()
                    .unwrap()
            })
            .collect::<Vec<_>>();

        // add lines if line styling is present
        if let Some(line_style) = &self.line_style {
            for i in 0..self.data.len() - 1 {
                let start = self.data[i];
                let end = self.data[i + 1];
                let pos = LinePositioning::BetweenPoints { start, end };
                let line = Line::new(pos.convert_to_u32(), line_style.clone());
                mask_points.extend(line.get_mask()?);
            }
        };

        Ok(mask_points)
    }
}

impl<T, U> Scalable<T, U> for Series<T>
where
    T: FloatConvertable + Graphable,
    U: FloatConvertable + Graphable,
{
    type ScaleTo = Series<f64>;
    fn scale_to(self, old_limits: &Limits<T>, new_limits: &Limits<U>) -> Self::ScaleTo {
        let scaled_data = self
            .data
            .iter()
            .map(|p| p.scale_to(old_limits, new_limits))
            .collect::<Vec<_>>();

        self.clone_with(&scaled_data)
    }
}

impl<T> Shiftable<T> for Series<T>
where
    T: FloatConvertable + Graphable,
{
    fn shift_by(mut self, amount: Point<T>) -> Self {
        self.data = self.data.iter().map(|&p| p + amount).collect::<Vec<_>>();
        self
    }
}

impl<T: Graphable> Add<T> for Series<T> {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        let data: Vec<Point<T>> = self.data.iter().map(|&p| p + rhs).collect();
        self.clone_with(&data)
    }
}

impl<T: Graphable> Sub<T> for Series<T> {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        let data: Vec<Point<T>> = self.data.iter().map(|&p| p - rhs).collect();
        self.clone_with(&data)
    }
}

impl<T: Graphable> Mul<T> for Series<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        let data: Vec<Point<T>> = self.data.iter().map(|&p| p * rhs).collect();
        self.clone_with(&data)
    }
}

impl<T: Graphable> Div<T> for Series<T> {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        let data: Vec<Point<T>> = self.data.iter().map(|&p| p / rhs).collect();
        self.clone_with(&data)
    }
}

impl<T: Graphable> Add<Point<T>> for Series<T> {
    type Output = Self;

    fn add(self, rhs: Point<T>) -> Self::Output {
        let data: Vec<Point<T>> = self.data.iter().map(|&p| p + rhs).collect();
        self.clone_with(&data)
    }
}

impl<T: Graphable> Sub<Point<T>> for Series<T> {
    type Output = Self;

    fn sub(self, rhs: Point<T>) -> Self::Output {
        let data: Vec<Point<T>> = self.data.iter().map(|&p| p - rhs).collect();
        self.clone_with(&data)
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
        let data = vec![p1, p2, p3];
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
        let data = vec![p1, p2, p3];
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
        let data = vec![p1, p2, p3];
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
        let data = vec![p1, p2, p3];
        let s1 = Series::new(&data);
        let x = 2.0;

        let s2 = s1 / x;
        assert_eq!(s2.data[0], Point { x: 5.0, y: 10.0 });
        assert_eq!(s2.data[1], Point { x: 6.25, y: 8.75 });
        assert_eq!(s2.data[2], Point { x: 7.5, y: 7.5 });
    }

    #[test]
    #[should_panic]
    fn create_empty_series() {
        let data: Vec<Point<f32>> = vec![];
        Series::new(&data);
    }
}
