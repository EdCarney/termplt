use super::{
    common::{Drawable, Graphable, MaskPoints},
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
    limits: Option<Limits<T>>,
    //xy_axes: (Option<Line>, Option<Line>),
}

impl<T: Graphable> Graph<T> {
    pub fn new() -> Graph<T> {
        Graph {
            data: vec![],
            limits: None,
            //xy_axes: (None, None),
        }
    }

    pub fn with_series(mut self, series: Series<T>) -> Self {
        self.update_limits(series.data());
        self.data.push(series);
        self
    }

    fn update_limits(&mut self, data: &[Point<T>]) {
        match data.limits() {
            None => (),
            Some(data_limits) => {
                self.limits = match self.limits {
                    None => data.limits(),
                    Some(ref current_limits) => [
                        *current_limits.min(),
                        *data_limits.min(),
                        *current_limits.max(),
                        *data_limits.max(),
                    ]
                    .as_slice()
                    .limits(),
                }
            }
        }
    }

    pub fn data(&self) -> &[Series<T>] {
        &self.data
    }

    pub fn limits(&self) -> Option<&Limits<T>> {
        match self.limits {
            Some(ref limits) => Some(limits),
            None => None,
        }
    }

    pub fn scale<U>(&self, limits: Limits<U>, convert_fn: unsafe fn(f64) -> U) -> Graph<U>
    where
        T: Into<f64>,
        U: Graphable + Into<f64>,
    {
        let data_limits = self.limits().expect("Cannot scale an empty graph");
        let (data_span_x, data_span_y) = data_limits.span();
        let data_span_x: f64 = data_span_x.into();
        let data_span_y: f64 = data_span_y.into();

        let (canvas_span_x, canvas_span_y) = limits.span();
        let canvas_span_x: f64 = canvas_span_x.into();
        let canvas_span_y: f64 = canvas_span_y.into();

        let scaled_data = self
            .data
            .iter()
            .map(|series| {
                let scaled_data = series
                    .data()
                    .iter()
                    .map(|p| {
                        let p = *p - *data_limits.min();

                        let x: f64 = p.x.into();
                        let y: f64 = p.y.into();

                        let x = x * canvas_span_x / data_span_x;
                        let y = y * canvas_span_y / data_span_y;

                        let x: U = unsafe { convert_fn(x) };
                        let y: U = unsafe { convert_fn(y) };

                        Point { x, y } + *limits.min()
                    })
                    .collect::<Vec<_>>();
                Series::new(&scaled_data)
            })
            .collect::<Vec<_>>();

        Graph {
            data: scaled_data,
            limits: Some(limits),
        }
    }
}

impl Drawable for Graph<u32> {
    fn bounding_width(&self) -> u32 {
        self.limits().unwrap().span().0
    }

    fn bounding_height(&self) -> u32 {
        self.limits().unwrap().span().1
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
            *limits.unwrap(),
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
            *limits.unwrap(),
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
            *limits.unwrap(),
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
            *limits.unwrap(),
            Limits::new(Point::new(-20, -50), Point::new(100, 50))
        );
    }
}
