use super::{
    common::{Convertable, FloatConvertable, Graphable},
    point::Point,
};

#[derive(Debug, PartialEq, Clone)]
pub struct Limits<T: Graphable> {
    min: Point<T>,
    max: Point<T>,
}

impl<T: Graphable, U: Graphable> Convertable<U> for Limits<T> {
    type ConvertTo = Limits<U>;
    fn convert_to(&self, convert_fn: fn(f64) -> U) -> Self::ConvertTo {
        Limits {
            min: self.min().convert_to(convert_fn),
            max: self.max().convert_to(convert_fn),
        }
    }
}

impl<T: FloatConvertable + Graphable> Limits<T> {
    /// Creates a new instance with the specified min/max. Requires that the max point be greater
    /// than or equal to the min point in both dimensions.
    ///
    /// # Panics
    ///
    /// Panics if `min` exceeds `max` in either dimension; use [`Limits::try_new`] to validate
    /// untrusted input.
    pub fn new(min: Point<T>, max: Point<T>) -> Limits<T> {
        Self::validate_limit(&min, &max);
        Limits { min, max }
    }

    /// Creates a new instance, returning an error if `min` exceeds `max` in either dimension or
    /// if either point is not comparable (e.g. contains NaN).
    pub fn try_new(min: Point<T>, max: Point<T>) -> crate::common::Result<Limits<T>> {
        if min.x <= max.x && min.y <= max.y {
            Ok(Limits { min, max })
        } else {
            Err(format!("Invalid limits: min {min:?} must not exceed max {max:?}").into())
        }
    }

    pub fn update_min(&mut self, new_min: Point<T>) {
        Self::validate_limit(&new_min, &self.max);
        self.min = new_min;
    }

    pub fn update_max(&mut self, new_max: Point<T>) {
        Self::validate_limit(&self.min, &new_max);
        self.max = new_max;
    }

    fn validate_limit(min: &Point<T>, max: &Point<T>) {
        if min.x > max.x || min.y > max.y {
            panic!(
                "Span between {:?} and {:?} must not be negative in any dimension",
                min, max
            );
        }
    }

    pub fn span(&self) -> (T, T) {
        let diff = self.max - self.min;
        (diff.x, diff.y)
    }

    pub fn min(&self) -> &Point<T> {
        &self.min
    }

    pub fn max(&self) -> &Point<T> {
        &self.max
    }

    pub fn upper_left(&self) -> Point<T> {
        Point::new(self.min().x, self.max().y)
    }

    pub fn upper_right(&self) -> Point<T> {
        *self.max()
    }

    pub fn lower_right(&self) -> Point<T> {
        Point::new(self.max().x, self.min().y)
    }

    pub fn lower_left(&self) -> Point<T> {
        *self.min()
    }

    /// Validates whether the provided point exists within the limit.
    pub fn contains(&self, point: &Point<T>) -> bool {
        (self.min.x..=self.max.x).contains(&point.x) && (self.min.y..=self.max.y).contains(&point.y)
    }

    /// Checks if this limit intersects with another limit (touching edges count). Two
    /// rectangles overlap exactly when their ranges overlap on both axes; testing corners alone
    /// misses cross-shaped overlaps, where no corner of either lies inside the other.
    pub fn intersects<U: FloatConvertable + Graphable>(&self, other: Limits<U>) -> bool {
        let this = self.convert_to_f64();
        let other = other.convert_to_f64();

        this.min.x <= other.max.x
            && other.min.x <= this.max.x
            && this.min.y <= other.max.y
            && other.min.y <= this.max.y
    }

    /// Chunks the limits into a collection of x and y points that will split the limit range into
    /// num_chunks^2 sections.
    pub fn chunk(&self, num_chunks: u32) -> (Vec<Point<f64>>, Vec<Point<f64>>) {
        let limits = self.convert_to_f64();
        let (limit_span_x, limit_span_y) = limits.span();

        let interval_x = limit_span_x / num_chunks.convert_to_f64();
        let interval_y = limit_span_y / num_chunks.convert_to_f64();

        let x_points = (0..=num_chunks)
            .map(|i| *limits.min() + Point::new(interval_x * i as f64, 0.))
            .collect::<Vec<_>>();
        let y_points = (0..=num_chunks)
            .map(|i| *limits.min() + Point::new(0., interval_y * i as f64))
            .collect::<Vec<_>>();

        (x_points, y_points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_new_validates_range() {
        assert!(Limits::try_new(Point::new(0, 0), Point::new(1, 1)).is_ok());
        assert!(Limits::try_new(Point::new(0, 0), Point::new(0, 0)).is_ok());
        assert!(Limits::try_new(Point::new(2, 0), Point::new(1, 1)).is_err());
        assert!(Limits::try_new(Point::new(0.0, f64::NAN), Point::new(1.0, 1.0)).is_err());
    }

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
        assert_eq!(limits.span(), (10, 5));

        limits.update_min(Point { x: 1, y: 1 });
        assert_eq!(limits.span(), (9, 4));
    }

    #[test]
    #[should_panic]
    fn update_min_invalid() {
        let min = Point { x: 0, y: 0 };
        let max = Point { x: 10, y: 5 };
        let mut limits = Limits::new(min, max);
        assert_eq!(limits.span(), (10, 5));

        limits.update_min(Point { x: 11, y: 1 });
    }

    #[test]
    fn update_max_valid() {
        let min = Point { x: 0, y: 0 };
        let max = Point { x: 10, y: 5 };
        let mut limits = Limits::new(min, max);
        assert_eq!(limits.span(), (10, 5));

        limits.update_max(Point { x: 15, y: 10 });
        assert_eq!(limits.span(), (15, 10));
    }

    #[test]
    #[should_panic]
    fn update_max_invalid() {
        let min = Point { x: 0, y: 0 };
        let max = Point { x: 10, y: 5 };
        let mut limits = Limits::new(min, max);
        assert_eq!(limits.span(), (10, 5));

        limits.update_max(Point { x: -1, y: 10 });
    }

    #[test]
    fn point_contained_in_limits() {
        let min = Point { x: 0, y: 0 };
        let max = Point { x: 10, y: 5 };
        let limits = Limits::new(min, max);

        for x in 0..=10 {
            for y in 0..=5 {
                assert!(limits.contains(&Point { x, y }));
            }
        }
    }

    #[test]
    fn point_not_contained_in_limits() {
        let min = Point { x: 0, y: 0 };
        let max = Point { x: 10, y: 5 };
        let limits = Limits::new(min, max);

        assert!(!limits.contains(&Point { x: -1, y: 1 }));
        assert!(!limits.contains(&Point { x: 1, y: -1 }));
        assert!(!limits.contains(&Point { x: 11, y: 4 }));
        assert!(!limits.contains(&Point { x: 9, y: 6 }));
    }

    #[test]
    fn intersects_detects_cross_shaped_overlap() {
        let wide = Limits::new(Point::new(0, 4), Point::new(10, 6));
        let tall = Limits::new(Point::new(4, 0), Point::new(6, 10));
        assert!(wide.intersects(tall.clone()));
        assert!(tall.intersects(wide.clone()));

        let apart = Limits::new(Point::new(11, 0), Point::new(12, 10));
        assert!(!wide.intersects(apart));
        let touching = Limits::new(Point::new(10, 0), Point::new(12, 4));
        assert!(wide.intersects(touching));
    }
}
