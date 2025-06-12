use super::{
    common::{Convertable, Drawable, FloatConvertable, Graphable, MaskPoints, UIntConvertable},
    limits::Limits,
    line::{Line, LineStyle},
    point::{Point, PointCollection},
    series::Series,
};
use crate::common::Result;

// TODO: implement items like: axis inclusion, grid lines, legends, etc.
#[derive(Debug)]
pub struct Graph<T: Graphable> {
    data: Vec<Series<T>>,
    x_axis: Option<Line<T>>,
    y_axis: Option<Line<T>>,
}

impl<T: Graphable, U: Graphable> Convertable<U> for Graph<T> {
    type ConvertTo = Graph<U>;
    fn convert_to(&self, convert_fn: unsafe fn(f64) -> U) -> Self::ConvertTo {
        let data = self
            .data
            .iter()
            .map(|series| series.convert_to(convert_fn))
            .collect::<Vec<_>>();
        let x_axis = if let Some(value) = &self.x_axis {
            Some(value.convert_to(convert_fn))
        } else {
            None
        };
        let y_axis = if let Some(value) = &self.y_axis {
            Some(value.convert_to(convert_fn))
        } else {
            None
        };
        Graph {
            data,
            x_axis,
            y_axis,
        }
    }
}

impl<T: Graphable> Graph<T> {
    pub fn new() -> Graph<T> {
        Graph {
            data: vec![],
            x_axis: None,
            y_axis: None,
        }
    }

    pub fn with_series(mut self, series: Series<T>) -> Self {
        self.data.push(series);
        self
    }

    pub fn data(&self) -> &[Series<T>] {
        &self.data
    }

    pub fn limits(&self) -> Option<Limits<T>> {
        self.data
            .iter()
            .flat_map(|series| series.data().to_vec())
            .collect::<Vec<Point<_>>>()
            .as_slice()
            .limits()
    }

    pub fn scale(mut self, limits: Limits<T>) -> Graph<f64> {
        let old_limits = self
            .limits()
            .expect("Cannot scale an empty graph")
            .convert_to_f64();
        let (old_span_x, old_span_y) = old_limits.span();

        let new_limits = limits.convert_to_f64();
        let (new_span_x, new_span_y) = new_limits.span();

        let x_factor = new_span_x / old_span_x;
        let y_factor = new_span_y / old_span_y;

        let data = self
            .data
            .iter()
            .map(|series| {
                series
                    .convert_to_f64()
                    .shift(*old_limits.min() * -1.)
                    .scale(x_factor, y_factor)
                    .shift(*new_limits.min())
            })
            .collect::<Vec<_>>();

        let x_axis = match self.x_axis.take() {
            None => None,
            Some(line) => Some(line.convert_to_f64()),
        };

        let y_axis = match self.y_axis.take() {
            None => None,
            Some(line) => Some(line.convert_to_f64()),
        };

        Graph {
            data,
            x_axis,
            y_axis,
        }
    }
}

impl<T: UIntConvertable + Graphable> Graph<T> {
    pub fn drawable_limits(&self) -> Option<Limits<u32>> {
        let data_limits = self
            .data
            .iter()
            .flat_map(|series| series.data().to_vec())
            .collect::<Vec<Point<_>>>()
            .as_slice()
            .limits()?
            .convert_to_u32();

        let mut min_x = data_limits.min().x;
        let mut min_y = data_limits.min().y;
        let mut max_x = data_limits.max().x;
        let mut max_y = data_limits.max().y;

        if let Some(x_axis) = &self.x_axis {
            let limits = x_axis.drawable_limits();
            min_x = limits.min().x;
            max_x = limits.max().x;
        }

        if let Some(y_axis) = &self.y_axis {
            let limits = y_axis.drawable_limits().convert_to_u32();
            min_y = limits.min().y;
            max_y = limits.max().y;
        }

        Some(Limits::new(
            Point::new(min_x, min_y),
            Point::new(max_x, max_y),
        ))
    }
}

impl<T: UIntConvertable + Graphable> Drawable for Graph<T> {
    fn bounding_width(&self) -> u32 {
        self.drawable_limits().unwrap().span().0
    }

    fn bounding_height(&self) -> u32 {
        self.drawable_limits().unwrap().span().1
    }

    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let mut mask_points = self
            .data()
            .iter()
            .flat_map(|series| series.get_mask().unwrap())
            .collect::<Vec<_>>();

        // add axes if they are defined

        Ok(mask_points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph() {
        let g = Graph::<u32>::new();
        assert!(g.limits().is_none());
    }

    #[test]
    fn add_single_series_with_single_point() {
        let g = Graph::new().with_series(Series::new(&vec![Point::new(0, 0)]));

        let limits = g.limits();
        assert!(limits.is_some());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(0, 0), Point::new(0, 0))
        );
    }

    #[test]
    fn add_single_series_with_multiple_points() {
        let g = Graph::new().with_series(Series::new(&vec![
            Point::new(0, -5),
            Point::new(10, 0),
            Point::new(-1, 15),
        ]));

        let limits = g.limits();
        assert!(limits.is_some());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(-1, -5), Point::new(10, 15))
        );
    }

    #[test]
    fn add_multiple_series_with_single_points() {
        let g = Graph::new()
            .with_series(Series::new(&vec![Point::new(0, -5)]))
            .with_series(Series::new(&vec![Point::new(10, 0)]))
            .with_series(Series::new(&vec![Point::new(-1, 15)]));

        let limits = g.limits();
        assert!(limits.is_some());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(-1, -5), Point::new(10, 15))
        );
    }

    #[test]
    fn add_multiple_series_with_multiple_points() {
        let g = Graph::new()
            .with_series(Series::new(&vec![
                Point::new(10, -5),
                Point::new(0, -50),
                Point::new(-1, -1),
            ]))
            .with_series(Series::new(&vec![Point::new(-20, 0), Point::new(0, -5)]))
            .with_series(Series::new(&vec![
                Point::new(-1, 50),
                Point::new(2, -5),
                Point::new(3, -5),
                Point::new(100, -5),
            ]));

        let limits = g.limits();
        assert!(limits.is_some());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(-20, -50), Point::new(100, 50))
        );
    }
}
