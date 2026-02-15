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

        // When old_span is 0 (all points identical in that dimension),
        // map to the midpoint of the new range instead of dividing by zero.
        let x_factor = if old_span_x == 0.0 {
            None
        } else {
            Some(new_span_x / old_span_x)
        };
        let y_factor = if old_span_y == 0.0 {
            None
        } else {
            Some(new_span_y / old_span_y)
        };

        let new_mid_x = (new_limits.min().x + new_limits.max().x) / 2.0;
        let new_mid_y = (new_limits.min().y + new_limits.max().y) / 2.0;

        match self.convert_to_f64() {
            GraphLimits::XOnly { min, max } => GraphLimits::XOnly {
                min: x_factor.map_or(new_mid_x, |f| min * f),
                max: x_factor.map_or(new_mid_x, |f| max * f),
            },
            GraphLimits::YOnly { min, max } => GraphLimits::YOnly {
                min: y_factor.map_or(new_mid_y, |f| min * f),
                max: y_factor.map_or(new_mid_y, |f| max * f),
            },
            GraphLimits::XY { min, max } => GraphLimits::XY {
                min: min.scale_to(&old_limits, &new_limits),
                max: max.scale_to(&old_limits, &new_limits),
            },
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn old_limits(x_min: f64, y_min: f64, x_max: f64, y_max: f64) -> Limits<f64> {
        Limits::new(Point::new(x_min, y_min), Point::new(x_max, y_max))
    }

    fn new_limits(x_min: f64, y_min: f64, x_max: f64, y_max: f64) -> Limits<f64> {
        Limits::new(Point::new(x_min, y_min), Point::new(x_max, y_max))
    }

    // --- scale_to tests ---

    #[test]
    fn scale_to_x_only_normal() {
        let gl = GraphLimits::XOnly { min: 2.0, max: 8.0 };
        let old = old_limits(0.0, 0.0, 10.0, 10.0);
        let new = new_limits(0.0, 0.0, 100.0, 100.0);

        let scaled = gl.scale_to(&old, &new);
        match scaled {
            GraphLimits::XOnly { min, max } => {
                assert_eq!(min, 20.0);
                assert_eq!(max, 80.0);
            }
            _ => panic!("Expected XOnly variant"),
        }
    }

    #[test]
    fn scale_to_y_only_normal() {
        let gl = GraphLimits::YOnly { min: 2.0, max: 8.0 };
        let old = old_limits(0.0, 0.0, 10.0, 10.0);
        let new = new_limits(0.0, 0.0, 100.0, 100.0);

        let scaled = gl.scale_to(&old, &new);
        match scaled {
            GraphLimits::YOnly { min, max } => {
                assert_eq!(min, 20.0);
                assert_eq!(max, 80.0);
            }
            _ => panic!("Expected YOnly variant"),
        }
    }

    #[test]
    fn scale_to_xy_normal() {
        let gl = GraphLimits::XY {
            min: Point::new(2.0, 3.0),
            max: Point::new(8.0, 7.0),
        };
        let old = old_limits(0.0, 0.0, 10.0, 10.0);
        let new = new_limits(0.0, 0.0, 100.0, 100.0);

        let scaled = gl.scale_to(&old, &new);
        match scaled {
            GraphLimits::XY { min, max } => {
                assert_eq!(min.x, 20.0);
                assert_eq!(min.y, 30.0);
                assert_eq!(max.x, 80.0);
                assert_eq!(max.y, 70.0);
            }
            _ => panic!("Expected XY variant"),
        }
    }

    #[test]
    fn scale_to_x_only_zero_x_span_maps_to_midpoint() {
        // old_limits has zero x span — XOnly should fall back to midpoint of new x range
        let gl = GraphLimits::XOnly { min: 5.0, max: 5.0 };
        let old = old_limits(5.0, 0.0, 5.0, 10.0);
        let new = new_limits(0.0, 0.0, 100.0, 100.0);

        let scaled = gl.scale_to(&old, &new);
        match scaled {
            GraphLimits::XOnly { min, max } => {
                assert_eq!(min, 50.0, "Zero x-span XOnly min should map to midpoint");
                assert_eq!(max, 50.0, "Zero x-span XOnly max should map to midpoint");
            }
            _ => panic!("Expected XOnly variant"),
        }
    }

    #[test]
    fn scale_to_y_only_zero_y_span_maps_to_midpoint() {
        // old_limits has zero y span — YOnly should fall back to midpoint of new y range
        let gl = GraphLimits::YOnly { min: 5.0, max: 5.0 };
        let old = old_limits(0.0, 5.0, 10.0, 5.0);
        let new = new_limits(0.0, 0.0, 100.0, 100.0);

        let scaled = gl.scale_to(&old, &new);
        match scaled {
            GraphLimits::YOnly { min, max } => {
                assert_eq!(min, 50.0, "Zero y-span YOnly min should map to midpoint");
                assert_eq!(max, 50.0, "Zero y-span YOnly max should map to midpoint");
            }
            _ => panic!("Expected YOnly variant"),
        }
    }

    #[test]
    fn scale_to_xy_zero_both_spans_maps_to_midpoints() {
        // old_limits has zero span in both dimensions
        let gl = GraphLimits::XY {
            min: Point::new(5.0, 5.0),
            max: Point::new(5.0, 5.0),
        };
        let old = old_limits(5.0, 5.0, 5.0, 5.0);
        let new = new_limits(0.0, 0.0, 100.0, 100.0);

        let scaled = gl.scale_to(&old, &new);
        match scaled {
            GraphLimits::XY { min, max } => {
                assert_eq!(min.x, 50.0, "Zero x-span XY min.x should map to midpoint");
                assert_eq!(min.y, 50.0, "Zero y-span XY min.y should map to midpoint");
                assert_eq!(max.x, 50.0, "Zero x-span XY max.x should map to midpoint");
                assert_eq!(max.y, 50.0, "Zero y-span XY max.y should map to midpoint");
            }
            _ => panic!("Expected XY variant"),
        }
    }

    // --- shift_by tests ---

    #[test]
    fn shift_by_x_only() {
        let gl = GraphLimits::XOnly { min: 2.0, max: 8.0 };
        let shifted = gl.shift_by(Point::new(10.0, 5.0));
        match shifted {
            GraphLimits::XOnly { min, max } => {
                assert_eq!(min, 12.0);
                assert_eq!(max, 18.0);
            }
            _ => panic!("Expected XOnly variant"),
        }
    }

    #[test]
    fn shift_by_y_only() {
        let gl = GraphLimits::YOnly { min: 2.0, max: 8.0 };
        let shifted = gl.shift_by(Point::new(10.0, 5.0));
        match shifted {
            GraphLimits::YOnly { min, max } => {
                assert_eq!(min, 7.0);
                assert_eq!(max, 13.0);
            }
            _ => panic!("Expected YOnly variant"),
        }
    }

    #[test]
    fn shift_by_xy() {
        let gl = GraphLimits::XY {
            min: Point::new(2.0, 3.0),
            max: Point::new(8.0, 7.0),
        };
        let shifted = gl.shift_by(Point::new(10.0, 5.0));
        match shifted {
            GraphLimits::XY { min, max } => {
                assert_eq!(min, Point::new(12.0, 8.0));
                assert_eq!(max, Point::new(18.0, 12.0));
            }
            _ => panic!("Expected XY variant"),
        }
    }

    // --- convert_to tests ---

    #[test]
    fn convert_to_preserves_variant_x_only() {
        let gl: GraphLimits<f64> = GraphLimits::XOnly { min: 2.5, max: 7.5 };
        let converted: GraphLimits<i32> = gl.convert_to(|v| v as i32);
        match converted {
            GraphLimits::XOnly { min, max } => {
                assert_eq!(min, 2);
                assert_eq!(max, 7);
            }
            _ => panic!("Expected XOnly variant"),
        }
    }
}
