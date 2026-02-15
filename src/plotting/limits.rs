use super::{
    common::{Convertable, FloatConvertable, Graphable, Scalable, Shiftable},
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
    /// than the min point in both dimensions.
    pub fn new(min: Point<T>, max: Point<T>) -> Limits<T> {
        Self::validate_limit(&min, &max);
        Limits { min, max }
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
        self.max().clone()
    }

    pub fn lower_right(&self) -> Point<T> {
        Point::new(self.max().x, self.min().y)
    }

    pub fn lower_left(&self) -> Point<T> {
        self.min().clone()
    }

    /// Validates whether the provided point exists within the limit.
    pub fn contains(&self, point: &Point<T>) -> bool {
        (self.min.x..=self.max.x).contains(&point.x) && (self.min.y..=self.max.y).contains(&point.y)
    }

    /// Checks if this limit intersects with another limit.
    pub fn intersects<U: FloatConvertable + Graphable>(&self, other: Limits<U>) -> bool {
        let this = self.convert_to_f64();
        let other = other.convert_to_f64();

        this.contains(&other.upper_left())
            || this.contains(&other.upper_right())
            || this.contains(&other.lower_right())
            || this.contains(&other.lower_left())
            || other.contains(&this.upper_left())
            || other.contains(&this.upper_right())
            || other.contains(&this.lower_right())
            || other.contains(&this.lower_left())
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

impl<T, U> Scalable<T, U> for Limits<T>
where
    T: FloatConvertable + Graphable,
    U: FloatConvertable + Graphable,
{
    type ScaleTo = Limits<f64>;
    fn scale_to(self, old_limits: &Limits<T>, new_limits: &Limits<U>) -> Self::ScaleTo {
        let min = self.min.scale_to(old_limits, new_limits);
        let max = self.max.scale_to(old_limits, new_limits);
        Limits { min, max }
    }
}

impl<T> Shiftable<T> for Limits<T>
where
    T: FloatConvertable + Graphable,
{
    fn shift_by(self, amount: Point<T>) -> Self {
        let min = self.min + amount;
        let max = self.max + amount;
        Limits { min, max }
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
}
