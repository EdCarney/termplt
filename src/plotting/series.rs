use super::{
    canvas::Canvas,
    common::{Drawable, Graphable, MaskPoints, UIntConvertable},
    line::{Line, LineStamp, LineStyle, draw_segment},
    line_positioning::LinePositioning,
    marker::{Marker, MarkerStyle, draw_marker, marker_stamp},
    point::Point,
};
use crate::common::Result;
use std::ops::{Add, Div, Mul, Sub};

/// A sequence of data points with a marker style and an optional connecting line.
///
/// Values of any primitive numeric type are accepted and stored as `f64`:
///
/// ```
/// use termplt::plotting::{point::Point, series::Series};
///
/// let a = Series::new(&[Point::new(1, 2), Point::new(2, 4)]);
/// let b = Series::from_xy(&[1u64, 2, 3], &[0.5, 1.5, 1.0]);
/// let c: Series = (0..10).map(|i| (i, i * i)).collect();
/// let d = Series::from(vec![(1.0, 2.0), (3.0, 4.0)]);
/// assert_eq!(c.data()[3], Point::new(3.0, 9.0));
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Series {
    data: Vec<Point<f64>>,
    marker_style: MarkerStyle,
    line_style: Option<LineStyle>,
    label: Option<String>,
}

impl Series {
    /// Creates a series from the given points. An empty series is valid and draws nothing.
    pub fn new<T: Graphable>(data: &[Point<T>]) -> Series {
        data.iter().map(|p| (p.x, p.y)).collect()
    }

    /// Creates a series from separate x and y values, pairing them up in order. If the slices
    /// have different lengths, the extra values of the longer one are ignored.
    pub fn from_xy<X: Graphable, Y: Graphable>(xs: &[X], ys: &[Y]) -> Series {
        xs.iter().copied().zip(ys.iter().copied()).collect()
    }

    /// Creates a series from y values, using their indices (0, 1, 2, ...) as x values.
    pub fn from_y<Y: Graphable>(ys: &[Y]) -> Series {
        ys.iter().enumerate().map(|(i, &y)| (i as f64, y)).collect()
    }

    /// The points, in order.
    pub fn data(&self) -> &[Point<f64>] {
        &self.data
    }

    /// The marker drawn at each point.
    pub fn marker_style(&self) -> &MarkerStyle {
        &self.marker_style
    }

    /// The line connecting consecutive points, if any.
    pub fn line_style(&self) -> Option<&LineStyle> {
        self.line_style.as_ref()
    }

    /// Sets the marker drawn at each point ([`MarkerStyle::None`] for a plain line).
    pub fn with_marker_style(mut self, marker_style: MarkerStyle) -> Self {
        self.marker_style = marker_style;
        self
    }

    /// Connects consecutive points with a line.
    pub fn with_line_style(mut self, line_style: LineStyle) -> Self {
        self.line_style = Some(line_style);
        self
    }

    /// Names the series in the legend. An empty label is the same as none; series without a
    /// label are left out of the legend.
    pub fn with_label(mut self, text: impl Into<String>) -> Self {
        self.label = Some(text.into());
        self
    }

    /// The series' name in the legend; `None` when it has none, or an empty one.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref().filter(|label| !label.is_empty())
    }

    /// A series with the same styles and the points produced by `f`.
    pub(crate) fn map_points(&self, f: impl FnOnce(&[Point<f64>]) -> Vec<Point<f64>>) -> Series {
        Series {
            data: f(&self.data),
            marker_style: self.marker_style,
            line_style: self.line_style,
            label: self.label.clone(),
        }
    }
}

impl Series {
    /// Draws the series straight into `canvas`: the same pixels, in the same order, as
    /// [`Drawable::get_mask`], without allocating per marker or segment.
    pub(crate) fn draw_into(&self, canvas: &mut Canvas) -> Result<()> {
        if let Some(color) = self.marker_style.color() {
            let stamp = marker_stamp(&self.marker_style)?;
            for &p in self.data.iter().filter(|p| is_finite(p)) {
                draw_marker(canvas, p.convert_to_u32(), &stamp, color);
            }
        }

        if let Some(line_style) = &self.line_style {
            let stamp = LineStamp::new(line_style);
            let mut last = None;
            for pair in self.data.windows(2) {
                if !is_finite(&pair[0]) || !is_finite(&pair[1]) {
                    continue;
                }
                let segment = (pair[0].convert_to_u32(), pair[1].convert_to_u32());
                draw_segment(canvas, segment, line_style, &stamp, &mut last);
            }
        }
        Ok(())
    }
}

impl<X: Graphable, Y: Graphable> FromIterator<(X, Y)> for Series {
    fn from_iter<I: IntoIterator<Item = (X, Y)>>(iter: I) -> Series {
        Series {
            data: iter
                .into_iter()
                .map(|(x, y)| Point::new(x.to_f64(), y.to_f64()))
                .collect(),
            ..Series::default()
        }
    }
}

impl<X: Graphable, Y: Graphable> From<Vec<(X, Y)>> for Series {
    fn from(data: Vec<(X, Y)>) -> Series {
        data.into_iter().collect()
    }
}

impl<X: Graphable, Y: Graphable> From<&[(X, Y)]> for Series {
    fn from(data: &[(X, Y)]) -> Series {
        data.iter().copied().collect()
    }
}

impl<X: Graphable, Y: Graphable> From<(&[X], &[Y])> for Series {
    /// Pairs up x and y values like [`Series::from_xy`].
    fn from((xs, ys): (&[X], &[Y])) -> Series {
        Series::from_xy(xs, ys)
    }
}

impl<X: Graphable, Y: Graphable> From<(&Vec<X>, &Vec<Y>)> for Series {
    /// Pairs up x and y values like [`Series::from_xy`].
    fn from((xs, ys): (&Vec<X>, &Vec<Y>)) -> Series {
        Series::from_xy(xs, ys)
    }
}

impl<X: Graphable, Y: Graphable> From<(Vec<X>, Vec<Y>)> for Series {
    /// Pairs up x and y values like [`Series::from_xy`].
    fn from((xs, ys): (Vec<X>, Vec<Y>)) -> Series {
        Series::from_xy(&xs, &ys)
    }
}

impl<X: Graphable, Y: Graphable> From<&Vec<(X, Y)>> for Series {
    fn from(data: &Vec<(X, Y)>) -> Series {
        data.iter().copied().collect()
    }
}

impl<X: Graphable, Y: Graphable, const N: usize> From<[(X, Y); N]> for Series {
    fn from(data: [(X, Y); N]) -> Series {
        data.into_iter().collect()
    }
}

impl<X: Graphable, Y: Graphable, const N: usize> From<&[(X, Y); N]> for Series {
    fn from(data: &[(X, Y); N]) -> Series {
        data.iter().copied().collect()
    }
}

impl<X: Graphable, Y: Graphable, const N: usize, const M: usize> From<([X; N], [Y; M])> for Series {
    /// Pairs up x and y values like [`Series::from_xy`].
    fn from((xs, ys): ([X; N], [Y; M])) -> Series {
        Series::from_xy(&xs, &ys)
    }
}

impl<X: Graphable, Y: Graphable, const N: usize, const M: usize> From<(&[X; N], &[Y; M])>
    for Series
{
    /// Pairs up x and y values like [`Series::from_xy`].
    fn from((xs, ys): (&[X; N], &[Y; M])) -> Series {
        Series::from_xy(xs, ys)
    }
}

impl<T: Graphable> From<&[Point<T>]> for Series {
    fn from(data: &[Point<T>]) -> Series {
        Series::new(data)
    }
}

impl<T: Graphable> From<Vec<Point<T>>> for Series {
    fn from(data: Vec<Point<T>>) -> Series {
        Series::new(&data)
    }
}

/// Non-finite points (NaN, ±∞) are not drawn, and neither are the line segments touching them,
/// so they break the line.
fn is_finite(p: &Point<f64>) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

impl Drawable for Series {
    /// Draws the series in its own coordinates, which must already be pixel coordinates.
    /// Non-finite points are skipped and break the line.
    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let mut mask_points = Vec::new();
        for &p in self.data.iter().filter(|p| is_finite(p)) {
            mask_points.extend(Marker::new(p.convert_to_u32(), self.marker_style).get_mask()?);
        }

        // add lines if line styling is present
        if let Some(line_style) = &self.line_style {
            for pair in self.data.windows(2) {
                if !is_finite(&pair[0]) || !is_finite(&pair[1]) {
                    continue;
                }
                let (start, end) = (pair[0], pair[1]);
                let pos = LinePositioning::BetweenPoints { start, end };
                let line = Line::new(pos.convert_to_u32(), *line_style);
                mask_points.extend(line.get_mask()?);
            }
        };

        Ok(mask_points)
    }
}

macro_rules! impl_series_op {
    ($trait:ident, $method:ident, $rhs:ty) => {
        impl $trait<$rhs> for Series {
            type Output = Series;

            fn $method(self, rhs: $rhs) -> Series {
                self.map_points(|points| points.iter().map(|&p| p.$method(rhs)).collect())
            }
        }
    };
}

impl_series_op!(Add, add, f64);
impl_series_op!(Sub, sub, f64);
impl_series_op!(Mul, mul, f64);
impl_series_op!(Div, div, f64);
impl_series_op!(Add, add, Point<f64>);
impl_series_op!(Sub, sub, Point<f64>);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_round_trip_and_an_empty_one_is_none() {
        let series = Series::from(vec![(0, 0), (1, 1)]);
        assert_eq!(series.label(), None);
        assert_eq!(series.clone().with_label("sin").label(), Some("sin"));
        assert_eq!(series.with_label("").label(), None);
    }

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
    fn input_shapes_agree() {
        let expected = [Point::new(1.0, 10.0), Point::new(2.0, 20.0)];
        let shapes = [
            Series::new(&[Point::new(1u8, 10u8), Point::new(2, 20)]),
            Series::from_xy(&[1i64, 2], &[10.0f32, 20.0]),
            Series::from_xy(&[1usize, 2, 3], &[10u64, 20]),
            [(1, 10), (2, 20)].into_iter().collect(),
            Series::from(vec![(1.0, 10.0), (2.0, 20.0)]),
            Series::from(&[(1i16, 10u32), (2, 20)][..]),
            Series::from(vec![Point::new(1, 10), Point::new(2, 20)]),
        ];
        for series in shapes {
            assert_eq!(series.data(), expected);
        }
        assert_eq!(
            Series::from_y(&[5, 7]).data(),
            [Point::new(0.0, 5.0), Point::new(1.0, 7.0)]
        );
    }

    #[test]
    fn large_integers_are_accepted() {
        let big = 1u64 << 60;
        let series = Series::from_xy(&[big], &[i64::MIN]);
        assert_eq!(series.data()[0], Point::new(big as f64, i64::MIN as f64));
    }

    #[test]
    fn non_finite_points_break_the_line() {
        use crate::plotting::{canvas::Canvas, colors};
        let series = Series::new(&[
            Point::new(0.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(f64::NAN, f64::NAN),
            Point::new(30.0, 10.0),
            Point::new(39.0, 10.0),
        ])
        .with_marker_style(MarkerStyle::None)
        .with_line_style(LineStyle::solid(colors::LIME, 0));
        let mut canvas = Canvas::new(40, 20, colors::BLACK);
        series.draw_into(&mut canvas).unwrap();
        let bytes = canvas.get_bytes();
        let lit = |x: usize| bytes[(9 * 40 + x) * 3..][..3] != [0, 0, 0];
        // row 10 from the bottom is row 9 from the top
        assert!(lit(5) && lit(35), "segments on both sides are drawn");
        assert!(!lit(20), "nothing is drawn across the gap");
    }

    #[test]
    fn draw_into_matches_get_mask_on_random_data() {
        use crate::plotting::{canvas::Canvas, colors};
        // xorshift, so failures reproduce
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = move |n: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % n
        };
        for case in 0..300 {
            let points: Vec<Point<f64>> = (0..1 + next(12))
                .map(|_| match next(15) {
                    0 => Point::new(f64::NAN, 0.0),
                    // on and beyond the edges of the 40x40 canvas
                    _ => Point::new(next(60) as f64 - 10.0, next(60) as f64 - 10.0),
                })
                .collect();
            let (size, color) = (next(5) as u32, colors::RED);
            let marker = match next(5) {
                0 => MarkerStyle::None,
                1 => MarkerStyle::FilledSquare { size, color },
                2 => MarkerStyle::HollowSquare { size, color },
                3 => MarkerStyle::FilledCircle { size, color },
                _ => MarkerStyle::HollowCircle { size, color },
            };
            let thickness = next(4) as u32;
            let mut series = Series::new(&points).with_marker_style(marker);
            match next(3) {
                0 => {}
                1 => series = series.with_line_style(LineStyle::solid(colors::LIME, thickness)),
                _ => series = series.with_line_style(LineStyle::dashed(colors::LIME, thickness)),
            }
            let mut expected = Canvas::new(40, 40, colors::BLACK);
            for mask in series.get_mask().unwrap() {
                expected.set_pixels(&mask.points, &mask.color);
            }
            let mut actual = Canvas::new(40, 40, colors::BLACK);
            series.draw_into(&mut actual).unwrap();
            assert!(
                expected.get_bytes() == actual.get_bytes(),
                "case {case}: {series:?}"
            );
        }
    }

    #[test]
    fn draw_into_matches_get_mask() {
        use crate::plotting::{canvas::Canvas, colors};
        // includes points at and beyond the canvas edges, where offsets are clamped or dropped
        let points = [
            Point::new(0.0, 0.0),
            Point::new(1.0, 30.0),
            Point::new(20.4, 20.6),
            Point::new(21.0, 20.0),
            Point::new(21.0, 20.0),
            // a gap: neither this point nor the segments touching it are drawn
            Point::new(f64::NAN, 10.0),
            Point::new(39.0, 2.0),
            Point::new(45.0, 50.0),
            Point::new(3.0, 39.0),
        ];
        let markers = [
            MarkerStyle::None,
            MarkerStyle::FilledSquare {
                size: 0,
                color: colors::RED,
            },
            MarkerStyle::FilledSquare {
                size: 3,
                color: colors::RED,
            },
            MarkerStyle::HollowSquare {
                size: 2,
                color: colors::RED,
            },
            MarkerStyle::FilledCircle {
                size: 4,
                color: colors::RED,
            },
            MarkerStyle::HollowCircle {
                size: 3,
                color: colors::RED,
            },
        ];
        let lines = [
            None,
            Some(LineStyle::solid(colors::LIME, 0)),
            Some(LineStyle::solid(colors::LIME, 2)),
            Some(LineStyle::dashed(colors::LIME, 0)),
            Some(LineStyle::dashed(colors::LIME, 3)),
        ];
        for marker in markers {
            for line in lines {
                let mut series = Series::new(&points).with_marker_style(marker);
                if let Some(line) = line {
                    series = series.with_line_style(line);
                }
                let mut expected = Canvas::new(40, 40, colors::BLACK);
                for mask in series.get_mask().unwrap() {
                    expected.set_pixels(&mask.points, &mask.color);
                }
                let mut actual = Canvas::new(40, 40, colors::BLACK);
                series.draw_into(&mut actual).unwrap();
                assert!(
                    expected.get_bytes() == actual.get_bytes(),
                    "{marker:?} {line:?}"
                );
            }
        }
    }

    #[test]
    fn create_empty_series() {
        let data: Vec<Point<f32>> = vec![];
        let series = Series::new(&data).with_line_style(LineStyle::default());
        assert!(series.data().is_empty());
        assert!(series.get_mask().unwrap().is_empty());
    }
}
