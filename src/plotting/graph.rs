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

    pub fn limits(&self) -> &Limits<T> {
        match self.limits {
            Some(ref limits) => limits,
            None => panic!("No limits defined"),
        }
    }
}
