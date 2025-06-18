use super::{
    common::{
        Convertable, Drawable, FloatConvertable, Graphable, IntConvertable, MaskPoints, Scalable,
        Shiftable,
    },
    graph_limits::GraphLimits,
    limits::Limits,
    line::{Line, LineStyle},
    line_positioning::LinePositioning,
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

impl Axes {
    pub fn get_mask<T: FloatConvertable + Graphable>(
        &self,
        limits: Limits<T>,
    ) -> Result<Vec<MaskPoints>> {
        let limits = limits.convert_to_f64();
        let (limit_span_x, limit_span_y) = limits.span();

        // thickness is applied in both directions from the line center, so shift the start of
        // the line in the appropriate direction to ensure it will not go into the data area
        match self {
            Axes::XOnly(line_style) => {
                let start = Point::new(
                    limits.min().x,
                    limits.min().y - line_style.thickness().convert_to_f64(),
                );
                let length = limit_span_x;
                let pos = LinePositioning::Horizontal { start, length };
                Line::new(pos, *line_style).get_mask()
            }
            Axes::YOnly(line_style) => {
                let start = Point::new(
                    limits.min().x - line_style.thickness().convert_to_f64(),
                    limits.min().y,
                );
                let length = limit_span_y;
                let pos = LinePositioning::Vertical { start, length };
                Line::new(pos, *line_style).get_mask()
            }
            Axes::XY(line_style) => {
                let pos_x = LinePositioning::Horizontal {
                    start: Point::new(
                        limits.min().x,
                        limits.min().y - line_style.thickness().convert_to_f64(),
                    ),
                    length: limit_span_x,
                };
                let pos_y = LinePositioning::Vertical {
                    start: Point::new(
                        limits.min().x - line_style.thickness().convert_to_f64(),
                        limits.min().y,
                    ),
                    length: limit_span_y,
                };

                let mut mask = Line::new(pos_x, *line_style).get_mask()?;
                mask.extend(Line::new(pos_y, *line_style).get_mask()?);
                Ok(mask)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum GridLines {
    XOnly(LineStyle),
    YOnly(LineStyle),
    XY(LineStyle),
}

impl GridLines {
    pub fn get_mask<T: FloatConvertable + Graphable>(
        &self,
        limits: Limits<T>,
    ) -> Result<Vec<MaskPoints>> {
        let limits = limits.convert_to_f64();
        let (limit_span_x, limit_span_y) = limits.span();

        // assume 10 sections; [num lines] = [num sections] - 1
        let mut mask_points = Vec::new();
        let num_sections = 10;
        let interval_x = limit_span_x / num_sections.convert_to_f64();
        let interval_y = limit_span_y / num_sections.convert_to_f64();

        for i in 1..=num_sections {
            match self {
                GridLines::XOnly(line_style) => {
                    // start at 1 to skip the first line
                    let pos = LinePositioning::Horizontal {
                        start: Point::new(limits.min().x, interval_y * i as f64),
                        length: limit_span_x,
                    };
                    mask_points.extend(Line::new(pos, *line_style).get_mask()?);
                }
                GridLines::YOnly(line_style) => {
                    // start at 1 to skip the first line
                    let pos = LinePositioning::Vertical {
                        start: Point::new(interval_x * i as f64, limits.min().y),
                        length: limit_span_y,
                    };
                    mask_points.extend(Line::new(pos, *line_style).get_mask()?);
                }
                GridLines::XY(line_style) => {
                    let pos_horz = LinePositioning::Horizontal {
                        start: Point::new(limits.min().x, interval_y * i as f64),
                        length: limit_span_x,
                    };

                    let pos_vert = LinePositioning::Vertical {
                        start: Point::new(interval_x * i as f64, limits.min().y),
                        length: limit_span_y,
                    };

                    mask_points.extend(Line::new(pos_horz, *line_style).get_mask()?);
                    mask_points.extend(Line::new(pos_vert, *line_style).get_mask()?);
                }
            }
        }
        Ok(mask_points)
    }
}

// TODO: implement items like: grid lines, legends, etc.
#[derive(Debug, Clone)]
pub struct Graph<T: Graphable + FloatConvertable> {
    data: Vec<Series<T>>,
    graph_limits: Option<GraphLimits<T>>,
    axes: Option<Axes>,
    grid_lines: Option<GridLines>,
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
        let grid_lines = self.grid_lines.clone();

        Graph {
            data,
            graph_limits,
            axes,
            grid_lines,
        }
    }
}

impl<T: Graphable> Graph<T> {
    pub fn new() -> Graph<T> {
        Graph {
            data: vec![],
            graph_limits: None,
            axes: None,
            grid_lines: None,
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

    pub fn with_grid_lines(mut self, grid_lines: GridLines) -> Self {
        self.grid_lines = Some(grid_lines);
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

    pub fn with_y_limits(mut self, min: T, max: T) -> Self {
        let graph_limits = match self.graph_limits {
            None => GraphLimits::YOnly { min, max },
            Some(cur_lim) => match cur_lim {
                GraphLimits::YOnly { .. } => GraphLimits::YOnly { min, max },
                GraphLimits::XOnly {
                    min: x_min,
                    max: x_max,
                } => {
                    let min = Point::new(x_min, min);
                    let max = Point::new(x_max, max);
                    GraphLimits::XY { min, max }
                }
                GraphLimits::XY {
                    min: min_p,
                    max: max_p,
                } => {
                    let min = Point::new(min_p.x, min);
                    let max = Point::new(max_p.x, max);
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
        let mut scaled_graph = self.clone();

        let mut old_limits = self.limits().expect("Cannot scale an empty graph");
        // if there are explicit limits set; remove any points that don't lie within those limits
        if let Some(_) = self.graph_limits {
            scaled_graph.data = scaled_graph
                .data
                .iter()
                .map(|series| {
                    let mut filtered_data = Vec::new();
                    for p in series.data() {
                        if old_limits.contains(p) {
                            filtered_data.push(*p);
                        } else {
                            println!("WARN: point {p:?} outside of old limits {old_limits:?}");
                        }
                    }
                    series.clone_with(&filtered_data)
                })
                .collect::<Vec<_>>();

            // if any points were removed we need to update the limits, since the overall data set
            // limits may have changed
            old_limits = scaled_graph
                .limits()
                .expect("No valid points lie in specified graph limits");
        }
        scaled_graph.scale_to(&old_limits, &new_limits)
    }
}

impl<T: IntConvertable + Graphable> Drawable for Graph<T> {
    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let mut mask_points = Vec::new();

        // add axes if they are defined
        if let Some(axes) = &self.axes {
            let limits = self.limits().unwrap();
            mask_points.extend(axes.get_mask(limits)?);
        }

        // add grid lines if they are defined
        if let Some(grid_lines) = &self.grid_lines {
            let limits = self.limits().unwrap();
            mask_points.extend(grid_lines.get_mask(limits)?);
        }

        // add series data
        mask_points.extend(
            self.data()
                .iter()
                .flat_map(|series| series.get_mask().unwrap())
                .collect::<Vec<_>>(),
        );

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
            Some(graph_limits) => {
                let mut limits = graph_limits.convert_to_f64();
                limits = limits.shift_by(old_limit_shift);
                limits = limits.scale_to(&old_limits_f64, &new_limits_f64);
                limits = limits.shift_by(new_limit_shift);
                Some(limits)
            }
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
