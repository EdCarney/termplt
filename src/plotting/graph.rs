use super::{common::Graphable, limits::Limits, series::Series};

//TODO: implement items like: axis inclusion, grid lines, legends, etc.
pub struct Graph<T: Graphable<T>> {
    pub data: Vec<Series<T>>,
    pub data_limits: Limits<T>,
}
