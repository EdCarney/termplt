use super::{
    common::Graphable,
    limits::Limits,
    point::{Point, PointCollection},
    series::Series,
};

//TODO: implement items like: axis inclusion, grid lines, legends, etc.
pub struct Graph<T: Graphable<T>> {
    data: Vec<Series<T>>,
    limits: Option<Limits<T>>,
}

impl<T: Graphable<T>> Graph<T> {
    pub fn new() -> Graph<T> {
        Graph {
            data: vec![],
            limits: None,
        }
    }

    pub fn add_series(&mut self, series: Series<T>) {
        self.update_limits(series.data());
        self.data.push(series);
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
        let mut g = Graph::new();
        g.add_series(Series::new(&vec![Point::new(0, 0)]));

        let limits = g.limits();
        assert!(limits.is_some());
        assert_eq!(
            *limits.unwrap(),
            Limits::new(Point::new(0, 0), Point::new(0, 0))
        );
    }

    #[test]
    fn add_single_series_with_multiple_points() {
        let mut g = Graph::new();
        g.add_series(Series::new(&vec![
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
        let mut g = Graph::new();
        g.add_series(Series::new(&vec![Point::new(0, -5)]));
        g.add_series(Series::new(&vec![Point::new(10, 0)]));
        g.add_series(Series::new(&vec![Point::new(-1, 15)]));

        let limits = g.limits();
        assert!(limits.is_some());
        assert_eq!(
            *limits.unwrap(),
            Limits::new(Point::new(-1, -5), Point::new(10, 15))
        );
    }

    #[test]
    fn add_multiple_series_with_multiple_points() {
        let mut g = Graph::new();
        g.add_series(Series::new(&vec![
            Point::new(10, -5),
            Point::new(0, -50),
            Point::new(-1, -1),
        ]));
        g.add_series(Series::new(&vec![Point::new(-20, 0), Point::new(0, -5)]));
        g.add_series(Series::new(&vec![
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
