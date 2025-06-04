use std::ops::Sub;

use super::point::Point;

pub struct Limits<T> {
    pub min: Point<T>,
    pub max: Point<T>,
}

impl<T> Limits<T>
where
    T: Sub<Output = T> + Copy,
{
    pub fn span(&self) -> (T, T) {
        let diff = self.max - self.min;
        (diff.x, diff.y)
    }
}
