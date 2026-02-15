use super::{
    common::{Convertable, FloatConvertable, Graphable, Scalable, Shiftable},
    limits::Limits,
    point::Point,
};

#[derive(Debug, Clone)]
pub enum GraphLimits<T: FloatConvertable + Graphable> {
    XOnly { min: T, max: T },
    YOnly { min: T, max: T },
    XY { min: Point<T>, max: Point<T> },
}

impl<T> Shiftable<T> for GraphLimits<T>
where
    T: FloatConvertable + Graphable,
{
    fn shift_by(self, amount: Point<T>) -> Self {
        match self {
            Self::XOnly { min, max } => Self::XOnly {
                min: min + amount.x,
                max: max + amount.x,
            },
            Self::YOnly { min, max } => Self::YOnly {
                min: min + amount.y,
                max: max + amount.y,
            },
            Self::XY { min, max } => Self::XY {
                min: min + amount,
                max: max + amount,
            },
        }
    }
}

impl<T, U> Scalable<T, U> for GraphLimits<T>
where
    T: FloatConvertable + Graphable,
    U: FloatConvertable + Graphable,
{
    type ScaleTo = GraphLimits<f64>;
    fn scale_to(self, old_limits: &Limits<T>, new_limits: &Limits<U>) -> Self::ScaleTo {
        let old_limits = old_limits.convert_to_f64();
        let new_limits = new_limits.convert_to_f64();

        let (old_span_x, old_span_y) = old_limits.span();
        let (new_span_x, new_span_y) = new_limits.span();

        let x_factor = new_span_x / old_span_x;
        let y_factor = new_span_y / old_span_y;

        let scaled_limits = match self.convert_to_f64() {
            GraphLimits::XOnly { min, max } => GraphLimits::XOnly {
                min: min * x_factor,
                max: max * x_factor,
            },
            GraphLimits::YOnly { min, max } => GraphLimits::YOnly {
                min: min * y_factor,
                max: max * y_factor,
            },
            GraphLimits::XY { min, max } => GraphLimits::XY {
                min: min.scale_to(&old_limits, &new_limits),
                max: max.scale_to(&old_limits, &new_limits),
            },
        };
        scaled_limits
    }
}

impl<T: Graphable, U: Graphable> Convertable<U> for GraphLimits<T> {
    type ConvertTo = GraphLimits<U>;
    fn convert_to(&self, convert_fn: fn(f64) -> U) -> Self::ConvertTo {
        match &self {
            Self::XOnly { min, max } => GraphLimits::XOnly {
                min: min.convert_to(convert_fn),
                max: max.convert_to(convert_fn),
            },
            Self::YOnly { min, max } => GraphLimits::YOnly {
                min: min.convert_to(convert_fn),
                max: max.convert_to(convert_fn),
            },
            Self::XY { min, max } => GraphLimits::XY {
                min: min.convert_to(convert_fn),
                max: max.convert_to(convert_fn),
            },
        }
    }
}
