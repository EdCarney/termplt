use super::{common::Graphable, point::Point};

#[derive(Debug)]
pub struct Limits<T: Graphable<T>> {
    pub min: Point<T>,
    pub max: Point<T>,
}

impl<T: Graphable<T>> Limits<T> {
    pub fn get_span(&self) -> (T, T) {
        let diff = self.max - self.min;
        (diff.x, diff.y)
    }

    pub fn contains(&self, point: Point<T>) -> bool {
        (self.min.x..=self.max.x).contains(&point.x) && (self.min.y..=self.max.y).contains(&point.y)
    }
}
