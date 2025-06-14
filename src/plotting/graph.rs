use super::{
    common::{
        Convertable, Drawable, FloatConvertable, Graphable, MaskPoints, Scalable, Shiftable,
        UIntConvertable,
    },
    limits::Limits,
    line::{Line, LinePositioning, LineStyle},
    point::{Point, PointCollection},
    series::Series,
};
use crate::common::Result;

#[derive(Debug, Clone)]
pub enum Axes {
    XOnly(LineStyle),
    YOnly(LineStyle),
    XY(LineStyle),
}

#[derive(Debug, Clone)]
enum GraphLimits<T: FloatConvertable + Graphable> {
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

        match self.convert_to_f64() {
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
        }
    }
}

impl<T: Graphable, U: Graphable> Convertable<U> for GraphLimits<T> {
    type ConvertTo = GraphLimits<U>;
    fn convert_to(&self, convert_fn: unsafe fn(f64) -> U) -> Self::ConvertTo {
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

// TODO: implement items like: axis inclusion, grid lines, legends, etc.
#[derive(Debug)]
pub struct Graph<T: Graphable + FloatConvertable> {
    data: Vec<Series<T>>,
    graph_limits: Option<GraphLimits<T>>,
    axes: Option<Axes>,
}

impl<T: Graphable, U: Graphable> Convertable<U> for Graph<T> {
    type ConvertTo = Graph<U>;
    fn convert_to(&self, convert_fn: unsafe fn(f64) -> U) -> Self::ConvertTo {
        let data = self
            .data
            .iter()
            .map(|series| series.convert_to(convert_fn))
            .collect::<Vec<_>>();

        let graph_limits = if let Some(value) = &self.graph_limits {
            Some(value.convert_to(convert_fn))
        } else {
            None
        };

        let axes = self.axes.clone();

        Graph {
            data,
            graph_limits,
            axes,
        }
    }
}

impl<T: Graphable> Graph<T> {
    pub fn new() -> Graph<T> {
        Graph {
            data: vec![],
            graph_limits: None,
            axes: None,
        }
    }

    pub fn with_series(mut self, series: Series<T>) -> Self {
        self.data.push(series);
        self
    }

    pub fn with_axes(mut self, axes: Axes) -> Self {
        self.axes = Some(axes);
        self
    }

    pub fn with_x_limits(mut self, min: T, max: T) -> Self {
        let graph_limits = match self.graph_limits {
            None => GraphLimits::XOnly { min, max },
            Some(cur_lim) => match cur_lim {
                GraphLimits::XOnly { .. } => GraphLimits::XOnly { min, max },
                GraphLimits::YOnly {
                    min: y_min,
                    max: y_max,
                } => {
                    let min = Point::new(min, y_min);
                    let max = Point::new(max, y_max);
                    GraphLimits::XY { min, max }
                }
                GraphLimits::XY {
                    min: min_p,
                    max: max_p,
                } => {
                    let min = Point::new(min, min_p.y);
                    let max = Point::new(max, max_p.y);
                    GraphLimits::XY { min, max }
                }
            },
        };
        self.graph_limits = Some(graph_limits);
        self
    }

    pub fn data(&self) -> &[Series<T>] {
        &self.data
    }

    pub fn axes(&self) -> Option<Axes> {
        self.axes.clone()
    }

    pub fn limits(&self) -> Option<Limits<T>> {
        let mut limits = self
            .data
            .iter()
            .flat_map(|series| series.data().to_vec())
            .collect::<Vec<Point<_>>>()
            .as_slice()
            .limits()?;

        // explicit limits override data limits
        if let Some(graph_limits) = &self.graph_limits {
            match graph_limits {
                GraphLimits::XOnly {
                    min: x_min,
                    max: x_max,
                } => {
                    let min = Point::new(*x_min, limits.min().y);
                    let max = Point::new(*x_max, limits.max().y);
                    limits = Limits::new(min, max);
                }
                GraphLimits::YOnly {
                    min: y_min,
                    max: y_max,
                } => {
                    let min = Point::new(limits.min().x, *y_min);
                    let max = Point::new(limits.max().x, *y_max);
                    limits = Limits::new(min, max);
                }
                GraphLimits::XY { min, max } => {
                    limits = Limits::new(*min, *max);
                }
            }
        }

        Some(limits)
    }

    pub fn scale(self, new_limits: Limits<f64>) -> Graph<f64> {
        let old_limits = self.limits().expect("Cannot scale an empty graph");
        self.scale_to(&old_limits, &new_limits)
    }
}

impl<T: UIntConvertable + Graphable> Drawable for Graph<T> {
    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let mut mask_points = self
            .data()
            .iter()
            .flat_map(|series| series.get_mask().unwrap())
            .collect::<Vec<_>>();

        // add axes if they are defined
        let limits = self.limits().unwrap();
        let (limit_span_x, limit_span_y) = limits.span();
        if let Some(axes) = &self.axes {
            let line_mask = match axes {
                Axes::XOnly(line_style) => {
                    let pos = LinePositioning::Horizontal {
                        start: *limits.min(),
                        length: limit_span_x,
                    };
                    Line::new(pos, *line_style).get_mask()?
                }
                Axes::YOnly(line_style) => {
                    let pos = LinePositioning::Vertical {
                        start: *limits.min(),
                        length: limit_span_y,
                    };
                    Line::new(pos, *line_style).get_mask()?
                }
                Axes::XY(line_style) => {
                    let pos_x = LinePositioning::Horizontal {
                        start: *limits.min(),
                        length: limit_span_x,
                    };
                    let pos_y = LinePositioning::Vertical {
                        start: *limits.min(),
                        length: limit_span_y,
                    };
                    let mut mask = Line::new(pos_x, *line_style).get_mask()?;
                    mask.extend(Line::new(pos_y, *line_style).get_mask()?);
                    mask
                }
            };
            mask_points.extend(line_mask);
        }

        Ok(mask_points)
    }
}

impl<T, U> Scalable<T, U> for Graph<T>
where
    T: FloatConvertable + Graphable,
    U: FloatConvertable + Graphable,
{
    type ScaleTo = Graph<f64>;
    fn scale_to(self, old_limits: &Limits<T>, new_limits: &Limits<U>) -> Self::ScaleTo {
        let old_limits_f64 = old_limits.convert_to_f64();
        let new_limits_f64 = new_limits.convert_to_f64();

        let old_limit_shift = *old_limits_f64.min() * -1.;
        let new_limit_shift = *new_limits_f64.min();

        let mut scaled_graph = self.convert_to_f64();

        scaled_graph = scaled_graph.shift_by(old_limit_shift);
        scaled_graph.data = scaled_graph
            .data
            .into_iter()
            .map(|series| series.scale_to(&old_limits_f64, &new_limits_f64))
            .collect::<Vec<_>>();
        scaled_graph = scaled_graph.shift_by(new_limit_shift);

        scaled_graph.graph_limits = match self.graph_limits {
            Some(graph_limits) => Some(graph_limits.scale_to(old_limits, new_limits)),
            None => None,
        };

        scaled_graph.axes = self.axes.clone();

        scaled_graph
    }
}

impl<T> Shiftable<T> for Graph<T>
where
    T: FloatConvertable + Graphable,
{
    fn shift_by(mut self, amount: Point<T>) -> Self {
        self.data = self
            .data
            .into_iter()
            .map(|series| series.shift_by(amount))
            .collect::<Vec<_>>();

        self.graph_limits = match self.graph_limits {
            Some(graph_limits) => Some(graph_limits.shift_by(amount)),
            None => None,
        };

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph() {
        let g = Graph::<u32>::new();
        assert!(g.limits().is_none());
    }

    #[test]
    fn add_single_series_with_single_point() {
        let g = Graph::new().with_series(Series::new(&vec![Point::new(0, 0)]));

        let limits = g.limits();
        assert!(limits.is_some());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(0, 0), Point::new(0, 0))
        );
    }

    #[test]
    fn add_single_series_with_multiple_points() {
        let g = Graph::new().with_series(Series::new(&vec![
            Point::new(0, -5),
            Point::new(10, 0),
            Point::new(-1, 15),
        ]));

        let limits = g.limits();
        assert!(limits.is_some());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(-1, -5), Point::new(10, 15))
        );
    }

    #[test]
    fn add_multiple_series_with_single_points() {
        let g = Graph::new()
            .with_series(Series::new(&vec![Point::new(0, -5)]))
            .with_series(Series::new(&vec![Point::new(10, 0)]))
            .with_series(Series::new(&vec![Point::new(-1, 15)]));

        let limits = g.limits();
        assert!(limits.is_some());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(-1, -5), Point::new(10, 15))
        );
    }

    #[test]
    fn add_multiple_series_with_multiple_points() {
        let g = Graph::new()
            .with_series(Series::new(&vec![
                Point::new(10, -5),
                Point::new(0, -50),
                Point::new(-1, -1),
            ]))
            .with_series(Series::new(&vec![Point::new(-20, 0), Point::new(0, -5)]))
            .with_series(Series::new(&vec![
                Point::new(-1, 50),
                Point::new(2, -5),
                Point::new(3, -5),
                Point::new(100, -5),
            ]));

        let limits = g.limits();
        assert!(limits.is_some());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(-20, -50), Point::new(100, 50))
        );
    }
}
