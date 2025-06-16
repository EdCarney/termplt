use super::{
    common::{Convertable, FloatConvertable, Graphable, Scalable, Shiftable},
    limits::Limits,
    point::Point,
};

#[derive(Debug, Clone)]
pub enum LinePositioning<T: Graphable> {
    Horizontal { start: Point<T>, length: T },
    Vertical { start: Point<T>, length: T },
    BetweenPoints { start: Point<T>, end: Point<T> },
}

impl<T: Graphable> LinePositioning<T> {
    pub fn limits(&self) -> Limits<T> {
        let (min, max) = match self {
            LinePositioning::Horizontal { start, length } => {
                let start = start.clone();
                let end = Point::new(start.x + *length, start.y);
                (start, end)
            }
            LinePositioning::Vertical { start, length } => {
                let start = start.clone();
                let end = Point::new(start.x, start.y + *length);
                (start, end)
            }
            LinePositioning::BetweenPoints { start, end } => {
                // for thickness b/w points, assume that the thickness will not go outside the
                // limits defined by the two points
                (start.clone(), end.clone())
            }
        };
        Limits::new(min, max)
    }
}

impl<T: Graphable, U: Graphable> Convertable<U> for LinePositioning<T> {
    type ConvertTo = LinePositioning<U>;
    fn convert_to(&self, convert_fn: unsafe fn(f64) -> U) -> Self::ConvertTo {
        match &self {
            LinePositioning::Horizontal { start, length } => {
                let start = start.convert_to(convert_fn);
                let length = length.convert_to(convert_fn);
                LinePositioning::Horizontal { start, length }
            }
            LinePositioning::Vertical { start, length } => {
                let start = start.convert_to(convert_fn);
                let length = length.convert_to(convert_fn);
                LinePositioning::Vertical { start, length }
            }
            LinePositioning::BetweenPoints { start, end } => {
                let start = start.convert_to(convert_fn);
                let end = end.convert_to(convert_fn);
                LinePositioning::BetweenPoints { start, end }
            }
        }
    }
}

impl<T, U> Scalable<T, U> for LinePositioning<T>
where
    T: FloatConvertable + Graphable,
    U: FloatConvertable + Graphable,
{
    type ScaleTo = LinePositioning<f64>;
    fn scale_to(self, old_limits: &Limits<T>, new_limits: &Limits<U>) -> Self::ScaleTo {
        let limits = self.limits().scale_to(old_limits, new_limits);
        let start = *limits.min();
        let end = *limits.max();
        let length = end.dist(&start);
        let scaled_pos = match self {
            LinePositioning::Vertical { .. } => LinePositioning::Vertical { start, length },
            LinePositioning::Horizontal { .. } => LinePositioning::Horizontal { start, length },
            LinePositioning::BetweenPoints { .. } => LinePositioning::BetweenPoints { start, end },
        };
        scaled_pos
    }
}

impl<T> Shiftable<T> for LinePositioning<T>
where
    T: FloatConvertable + Graphable,
{
    fn shift_by(self, amount: Point<T>) -> Self {
        let limits = self.limits().shift_by(amount);
        let start = *limits.min();
        let end = *limits.max();
        match self {
            LinePositioning::Vertical { length, .. } => LinePositioning::Vertical { start, length },
            LinePositioning::Horizontal { length, .. } => {
                LinePositioning::Horizontal { start, length }
            }
            LinePositioning::BetweenPoints { .. } => LinePositioning::BetweenPoints { start, end },
        }
    }
}
