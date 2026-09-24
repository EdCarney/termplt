use super::{
    common::{Convertable, Graphable},
    limits::Limits,
    point::Point,
};

#[derive(Debug, Clone, Copy)]
pub enum LinePositioning<T: Graphable> {
    Horizontal { start: Point<T>, length: T },
    Vertical { start: Point<T>, length: T },
    BetweenPoints { start: Point<T>, end: Point<T> },
}

impl<T: Graphable> LinePositioning<T> {
    pub fn limits(&self) -> Limits<T> {
        let (min, max) = match self {
            LinePositioning::Horizontal { start, length } => {
                let start = *start;
                let end = Point::new(start.x + *length, start.y);
                (start, end)
            }
            LinePositioning::Vertical { start, length } => {
                let start = *start;
                let end = Point::new(start.x, start.y + *length);
                (start, end)
            }
            LinePositioning::BetweenPoints { start, end } => {
                // for thickness b/w points, assume that the thickness will not go outside the
                // limits defined by the two points
                (*start, *end)
            }
        };
        Limits::new(min, max)
    }
}

impl<T: Graphable, U: Graphable> Convertable<U> for LinePositioning<T> {
    type ConvertTo = LinePositioning<U>;
    fn convert_to(&self, convert_fn: fn(f64) -> U) -> Self::ConvertTo {
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
