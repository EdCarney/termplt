use super::{common::Graphable, point::Point};

#[derive(Debug, PartialEq)]
pub struct Limits<T: Graphable<T>> {
    min: Point<T>,
    max: Point<T>,
    span: (T, T),
}

impl<T: Graphable<T>> Limits<T> {
    /// Creates a new instance with the specified min/max. Requires that the max point be greater
    /// than the min point in both dimensions.
    pub fn new(min: Point<T>, max: Point<T>) -> Limits<T> {
        Self::validate_limit(&min, &max);
        let diff = max - min;
        Limits {
            min,
            max,
            span: (diff.x, diff.y),
        }
    }

    pub fn update_min(&mut self, new_min: Point<T>) {
        Self::validate_limit(&new_min, &self.max);
        let diff = self.max - new_min;
        self.min = new_min;
        self.span = (diff.x, diff.y);
    }

    pub fn update_max(&mut self, new_max: Point<T>) {
        Self::validate_limit(&self.min, &new_max);
        let diff = new_max - self.min;
        self.max = new_max;
        self.span = (diff.x, diff.y);
    }

    fn validate_limit(min: &Point<T>, max: &Point<T>) {
        if min.x >= max.x || min.y >= max.y {
            panic!(
                "Span between {:?} and {:?} must be nonzero in both dimensions",
                min, max
            );
        }
    }

    pub fn span(&self) -> &(T, T) {
        &self.span
    }

    pub fn min(&self) -> &Point<T> {
        &self.min
    }

    pub fn max(&self) -> &Point<T> {
        &self.max
    }

    /// Validates whether the provided point exists within the limit.
    pub fn contains(&self, point: Point<T>) -> bool {
        (self.min.x..=self.max.x).contains(&point.x) && (self.min.y..=self.max.y).contains(&point.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn create_with_invalid_range_x() {
        let min = Point { x: 5, y: 5 };
        let max = Point { x: 4, y: 7 };
        Limits::new(min, max);
    }

    #[test]
    #[should_panic]
    fn create_with_invalid_range_y() {
        let min = Point { x: 5, y: 5 };
        let max = Point { x: 7, y: 4 };
        Limits::new(min, max);
    }

    #[test]
    fn update_min_valid() {
        let min = Point { x: 0, y: 0 };
        let max = Point { x: 10, y: 5 };
        let mut limits = Limits::new(min, max);
        assert_eq!(*limits.span(), (10, 5));

        limits.update_min(Point { x: 1, y: 1 });
        assert_eq!(*limits.span(), (9, 4));
    }

    #[test]
    #[should_panic]
    fn update_min_invalid() {
        let min = Point { x: 0, y: 0 };
        let max = Point { x: 10, y: 5 };
        let mut limits = Limits::new(min, max);
        assert_eq!(*limits.span(), (10, 5));

        limits.update_min(Point { x: 10, y: 1 });
    }

    #[test]
    fn update_max_valid() {
        let min = Point { x: 0, y: 0 };
        let max = Point { x: 10, y: 5 };
        let mut limits = Limits::new(min, max);
        assert_eq!(*limits.span(), (10, 5));

        limits.update_max(Point { x: 15, y: 10 });
        assert_eq!(*limits.span(), (15, 10));
    }

    #[test]
    #[should_panic]
    fn update_max_invalid() {
        let min = Point { x: 0, y: 0 };
        let max = Point { x: 10, y: 5 };
        let mut limits = Limits::new(min, max);
        assert_eq!(*limits.span(), (10, 5));

        limits.update_max(Point { x: -1, y: 10 });
    }

    #[test]
    fn point_contained_in_limits() {
        let min = Point { x: 0, y: 0 };
        let max = Point { x: 10, y: 5 };
        let limits = Limits::new(min, max);

        for x in 0..=10 {
            for y in 0..=5 {
                assert!(limits.contains(Point { x, y }));
            }
        }
    }

    #[test]
    fn point_not_contained_in_limits() {
        let min = Point { x: 0, y: 0 };
        let max = Point { x: 10, y: 5 };
        let limits = Limits::new(min, max);

        assert!(!limits.contains(Point { x: -1, y: 1 }));
        assert!(!limits.contains(Point { x: 1, y: -1 }));
        assert!(!limits.contains(Point { x: 11, y: 4 }));
        assert!(!limits.contains(Point { x: 9, y: 6 }));
    }
}
