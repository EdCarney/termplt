use super::{
    axes::Axes,
    common::{
        Convertable, Drawable, FloatConvertable, Graphable, IntConvertable, MaskPoints, Scalable,
        Shiftable,
    },
    graph_limits::GraphLimits,
    grid_lines::GridLines,
    limits::Limits,
    point::{Point, PointCollection},
    series::Series,
};
use crate::common::Result;

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
    fn convert_to(&self, convert_fn: fn(f64) -> U) -> Self::ConvertTo {
        let data = self
            .data
            .iter()
            .map(|series| series.convert_to(convert_fn))
            .collect::<Vec<_>>();

        let graph_limits = self
            .graph_limits
            .as_ref()
            .map(|value| value.convert_to(convert_fn));

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

impl<T: Graphable> Default for Graph<T> {
    fn default() -> Self {
        Self::new()
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

    pub fn grid_lines(&self) -> Option<&GridLines> {
        self.grid_lines.as_ref()
    }

    /// Returns the plotted range: the extent of all finite data points, overridden by any
    /// explicit limits. Non-finite points (NaN, ±∞) are ignored.
    ///
    /// Errors if the graph has no finite data points or the explicit limits are invalid
    /// (inverted or non-finite).
    pub fn limits(&self) -> Result<Limits<T>> {
        self.validate_graph_limits()?;

        let mut limits = self
            .data
            .iter()
            .flat_map(|series| series.data().iter().copied())
            .filter(is_finite_point)
            .collect::<Vec<Point<_>>>()
            .as_slice()
            .limits()
            .ok_or("Graph has no data points to plot (non-finite values are ignored)")?;

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

        Ok(limits)
    }

    fn validate_graph_limits(&self) -> Result<()> {
        let check = |axis: &str, min: T, max: T| -> Result<()> {
            let (min, max): (f64, f64) = (min.into(), max.into());
            if !min.is_finite() || !max.is_finite() {
                return Err(format!("{axis} limits must be finite, got {min}..{max}").into());
            }
            if min > max {
                return Err(format!(
                    "{axis} limits are inverted: min ({min}) is greater than max ({max})"
                )
                .into());
            }
            Ok(())
        };

        match &self.graph_limits {
            None => Ok(()),
            Some(GraphLimits::XOnly { min, max }) => check("x", *min, *max),
            Some(GraphLimits::YOnly { min, max }) => check("y", *min, *max),
            Some(GraphLimits::XY { min, max }) => {
                check("x", min.x, max.x)?;
                check("y", min.y, max.y)
            }
        }
    }

    /// Returns a copy of the graph containing only the points that will be drawn: finite points
    /// that lie within the explicit limits (if any). Series left empty are kept.
    fn visible(&self) -> Result<Graph<T>> {
        let limits = self.limits()?;
        let clip = self.graph_limits.is_some();
        let data = self
            .data
            .iter()
            .map(|series| {
                let points = series
                    .data()
                    .iter()
                    .copied()
                    .filter(|p| is_finite_point(p) && (!clip || limits.contains(p)))
                    .collect::<Vec<_>>();
                series.clone_with(&points)
            })
            .collect();

        Ok(Graph {
            data,
            graph_limits: self.graph_limits.clone(),
            axes: self.axes.clone(),
            grid_lines: self.grid_lines.clone(),
        })
    }

    /// Returns the data range that will be mapped onto the drawable area: [`Graph::limits`]
    /// after clipping to explicit limits, with a margin of [`DATA_MARGIN`] of the span added on
    /// axes without explicit limits (so data does not touch the axes), and zero-width
    /// dimensions (e.g. a single point or a constant series) expanded so the data is centered.
    pub fn view_limits(&self) -> Result<Limits<f64>> {
        let limits = self
            .visible()?
            .limits()
            .map_err(|_| "No data points lie within the specified graph limits")?
            .convert_to_f64();

        let (explicit_x, explicit_y) = match &self.graph_limits {
            None => (false, false),
            Some(GraphLimits::XOnly { .. }) => (true, false),
            Some(GraphLimits::YOnly { .. }) => (false, true),
            Some(GraphLimits::XY { .. }) => (true, true),
        };
        let (span_x, span_y) = limits.span();
        let margin = Point::new(
            if explicit_x {
                0.0
            } else {
                span_x * DATA_MARGIN
            },
            if explicit_y {
                0.0
            } else {
                span_y * DATA_MARGIN
            },
        );
        let limits = pad_degenerate(Limits::new(*limits.min() - margin, *limits.max() + margin));

        let (span_x, span_y) = limits.span();
        if !span_x.is_finite() || !span_y.is_finite() {
            return Err("Data range is too large to plot (exceeds the range of f64)".into());
        }
        Ok(limits)
    }

    /// Scales the visible data so that [`Graph::view_limits`] maps onto `new_limits`.
    pub fn scale(self, new_limits: Limits<f64>) -> Result<Graph<f64>> {
        let view_limits = self.view_limits()?;
        self.scale_with_view(&view_limits, new_limits)
    }

    pub(crate) fn scale_with_view(
        &self,
        view_limits: &Limits<f64>,
        new_limits: Limits<f64>,
    ) -> Result<Graph<f64>> {
        let mut scaled_graph = self
            .visible()?
            .convert_to_f64()
            .scale_to(view_limits, &new_limits);

        // the view maps exactly onto the new limits; record them explicitly so the axes and grid
        // span the whole drawable area even where the data does not
        scaled_graph.graph_limits = Some(GraphLimits::XY {
            min: *new_limits.min(),
            max: *new_limits.max(),
        });
        Ok(scaled_graph)
    }
}

/// Fraction of the data span added on each side of axes without explicit limits.
pub const DATA_MARGIN: f64 = 0.05;

fn is_finite_point<T: Graphable>(p: &Point<T>) -> bool {
    let (x, y): (f64, f64) = (p.x.into(), p.y.into());
    x.is_finite() && y.is_finite()
}

/// Expands zero-width dimensions by 5% of the value (or ±0.5 around zero) so the value is drawn
/// at the center of the plot.
fn pad_degenerate(limits: Limits<f64>) -> Limits<f64> {
    let pad = |v: f64| if v == 0.0 { 0.5 } else { v.abs() * 0.05 };
    let mut min = *limits.min();
    let mut max = *limits.max();
    if min.x == max.x {
        let p = pad(min.x);
        min.x -= p;
        max.x += p;
    }
    if min.y == max.y {
        let p = pad(min.y);
        min.y -= p;
        max.y += p;
    }
    Limits::new(min, max)
}

impl<T: IntConvertable + Graphable> Drawable for Graph<T> {
    fn get_mask(&self) -> Result<Vec<MaskPoints>> {
        let mut mask_points = Vec::new();
        let limits = self.limits()?;

        // grid lines first so the axes and data are drawn over them
        if let Some(grid_lines) = &self.grid_lines {
            mask_points.extend(grid_lines.get_mask(&limits)?);
        }

        if let Some(axes) = &self.axes {
            mask_points.extend(axes.get_mask(&limits)?);
        }

        // add series data
        let series_masks: Vec<Vec<MaskPoints>> = self
            .data()
            .iter()
            .map(|series| series.get_mask())
            .collect::<Result<_>>()?;
        mask_points.extend(series_masks.into_iter().flatten());

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

        self.graph_limits = self
            .graph_limits
            .map(|graph_limits| graph_limits.shift_by(amount));

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph() {
        let g = Graph::<u32>::new();
        assert!(g.limits().is_err());
    }

    #[test]
    fn add_single_series_with_single_point() {
        let g = Graph::new().with_series(Series::new(&[Point::new(0, 0)]));

        let limits = g.limits();
        assert!(limits.is_ok());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(0, 0), Point::new(0, 0))
        );
    }

    #[test]
    fn add_single_series_with_multiple_points() {
        let g = Graph::new().with_series(Series::new(&[
            Point::new(0, -5),
            Point::new(10, 0),
            Point::new(-1, 15),
        ]));

        let limits = g.limits();
        assert!(limits.is_ok());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(-1, -5), Point::new(10, 15))
        );
    }

    #[test]
    fn add_multiple_series_with_single_points() {
        let g = Graph::new()
            .with_series(Series::new(&[Point::new(0, -5)]))
            .with_series(Series::new(&[Point::new(10, 0)]))
            .with_series(Series::new(&[Point::new(-1, 15)]));

        let limits = g.limits();
        assert!(limits.is_ok());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(-1, -5), Point::new(10, 15))
        );
    }

    #[test]
    fn add_multiple_series_with_multiple_points() {
        let g = Graph::new()
            .with_series(Series::new(&[
                Point::new(10, -5),
                Point::new(0, -50),
                Point::new(-1, -1),
            ]))
            .with_series(Series::new(&[Point::new(-20, 0), Point::new(0, -5)]))
            .with_series(Series::new(&[
                Point::new(-1, 50),
                Point::new(2, -5),
                Point::new(3, -5),
                Point::new(100, -5),
            ]));

        let limits = g.limits();
        assert!(limits.is_ok());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(-20, -50), Point::new(100, 50))
        );
    }

    // --- with_x_limits state machine ---

    fn graph_with_data() -> Graph<i32> {
        Graph::new().with_series(Series::new(&[Point::new(0, 0), Point::new(10, 20)]))
    }

    #[test]
    fn with_x_limits_from_none_produces_x_only() {
        // None → XOnly: x limits override data, y limits come from data
        let g = graph_with_data().with_x_limits(-5, 15);
        let limits = g.limits().unwrap();
        assert_eq!(limits.min().x, -5);
        assert_eq!(limits.max().x, 15);
        // y should still come from data
        assert_eq!(limits.min().y, 0);
        assert_eq!(limits.max().y, 20);
    }

    #[test]
    fn with_x_limits_from_x_only_replaces_x() {
        // XOnly → XOnly: new x limits replace old x limits
        let g = graph_with_data()
            .with_x_limits(-5, 15)
            .with_x_limits(-100, 100);
        let limits = g.limits().unwrap();
        assert_eq!(limits.min().x, -100);
        assert_eq!(limits.max().x, 100);
        assert_eq!(limits.min().y, 0);
        assert_eq!(limits.max().y, 20);
    }

    #[test]
    fn with_x_limits_from_y_only_produces_xy() {
        // YOnly → XY: combining x and y limits
        let g = graph_with_data()
            .with_y_limits(-10, 30)
            .with_x_limits(-5, 15);
        let limits = g.limits().unwrap();
        assert_eq!(limits.min().x, -5);
        assert_eq!(limits.max().x, 15);
        assert_eq!(limits.min().y, -10);
        assert_eq!(limits.max().y, 30);
    }

    #[test]
    fn with_x_limits_from_xy_updates_only_x() {
        // XY → XY: only x changes
        let g = graph_with_data()
            .with_x_limits(-5, 15)
            .with_y_limits(-10, 30)
            .with_x_limits(-50, 50);
        let limits = g.limits().unwrap();
        assert_eq!(limits.min().x, -50);
        assert_eq!(limits.max().x, 50);
        // y should be preserved from the earlier with_y_limits call
        assert_eq!(limits.min().y, -10);
        assert_eq!(limits.max().y, 30);
    }

    // --- with_y_limits state machine ---

    #[test]
    fn with_y_limits_from_none_produces_y_only() {
        // None → YOnly: y limits override data, x limits come from data
        let g = graph_with_data().with_y_limits(-10, 30);
        let limits = g.limits().unwrap();
        assert_eq!(limits.min().x, 0);
        assert_eq!(limits.max().x, 10);
        assert_eq!(limits.min().y, -10);
        assert_eq!(limits.max().y, 30);
    }

    #[test]
    fn with_y_limits_from_y_only_replaces_y() {
        // YOnly → YOnly: new y limits replace old y limits
        let g = graph_with_data()
            .with_y_limits(-10, 30)
            .with_y_limits(-100, 100);
        let limits = g.limits().unwrap();
        assert_eq!(limits.min().x, 0);
        assert_eq!(limits.max().x, 10);
        assert_eq!(limits.min().y, -100);
        assert_eq!(limits.max().y, 100);
    }

    #[test]
    fn with_y_limits_from_x_only_produces_xy() {
        // XOnly → XY: combining x and y limits
        let g = graph_with_data()
            .with_x_limits(-5, 15)
            .with_y_limits(-10, 30);
        let limits = g.limits().unwrap();
        assert_eq!(limits.min().x, -5);
        assert_eq!(limits.max().x, 15);
        assert_eq!(limits.min().y, -10);
        assert_eq!(limits.max().y, 30);
    }

    #[test]
    fn with_y_limits_from_xy_updates_only_y() {
        // XY → XY: only y changes
        let g = graph_with_data()
            .with_x_limits(-5, 15)
            .with_y_limits(-10, 30)
            .with_y_limits(-50, 50);
        let limits = g.limits().unwrap();
        // x should be preserved from the earlier with_x_limits call
        assert_eq!(limits.min().x, -5);
        assert_eq!(limits.max().x, 15);
        assert_eq!(limits.min().y, -50);
        assert_eq!(limits.max().y, 50);
    }

    // --- limits() with no data is an error even with graph_limits ---

    #[test]
    fn limits_with_no_data_returns_error() {
        let g = Graph::<i32>::new().with_x_limits(0, 10);
        assert!(
            g.limits().is_err(),
            "No data means no limits, even with explicit graph limits"
        );
    }

    #[test]
    fn get_mask_on_empty_graph_returns_error() {
        let g = Graph::<i32>::new();
        let result = g.get_mask();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no data"));
    }

    // --- non-finite data and invalid explicit limits ---

    #[test]
    fn limits_ignore_non_finite_points() {
        let g = Graph::new().with_series(Series::new(&[
            Point::new(f64::NAN, 0.0),
            Point::new(1.0, 1.0),
            Point::new(2.0, f64::INFINITY),
            Point::new(3.0, -2.0),
        ]));
        assert_eq!(
            g.limits().unwrap(),
            Limits::new(Point::new(1.0, -2.0), Point::new(3.0, 1.0))
        );
    }

    #[test]
    fn limits_with_only_non_finite_points_returns_error() {
        let g = Graph::new().with_series(Series::new(&[Point::new(f64::NAN, f64::NAN)]));
        assert!(g.limits().is_err());
    }

    #[test]
    fn inverted_explicit_limits_return_error() {
        let err = graph_with_data()
            .with_x_limits(15, -5)
            .limits()
            .unwrap_err();
        assert!(err.to_string().contains("inverted"), "{err}");
        let err = graph_with_data()
            .with_y_limits(30, -10)
            .limits()
            .unwrap_err();
        assert!(err.to_string().contains("inverted"), "{err}");
    }

    #[test]
    fn non_finite_explicit_limits_return_error() {
        let g = Graph::new()
            .with_series(Series::new(&[Point::new(0.0, 0.0), Point::new(1.0, 1.0)]))
            .with_x_limits(0.0, f64::NAN);
        assert!(g.limits().is_err());
    }

    // --- view limits and scaling ---

    #[test]
    fn view_limits_pad_single_point() {
        let g = Graph::new().with_series(Series::new(&[Point::new(10.0, 0.0)]));
        let view = g.view_limits().unwrap();
        assert_eq!(
            view,
            Limits::new(Point::new(9.5, -0.5), Point::new(10.5, 0.5))
        );
    }

    #[test]
    fn view_limits_add_margin_to_automatic_axes() {
        // data spans 0..10 x 0..20; 5% of each span is added on both sides
        let view = graph_with_data().view_limits().unwrap();
        assert_eq!(
            view,
            Limits::new(Point::new(-0.5, -1.0), Point::new(10.5, 21.0))
        );
    }

    #[test]
    fn view_limits_do_not_add_margin_to_explicit_axes() {
        let view = graph_with_data()
            .with_x_limits(0, 10)
            .view_limits()
            .unwrap();
        assert_eq!(
            view,
            Limits::new(Point::new(0.0, -1.0), Point::new(10.0, 21.0))
        );
        let view = graph_with_data()
            .with_x_limits(0, 10)
            .with_y_limits(0, 20)
            .view_limits()
            .unwrap();
        assert_eq!(
            view,
            Limits::new(Point::new(0.0, 0.0), Point::new(10.0, 20.0))
        );
    }

    #[test]
    fn scale_single_point_to_center_of_offset_limits() {
        let g = Graph::new().with_series(Series::new(&[Point::new(5.0, 5.0)]));
        let new_limits = Limits::new(Point::new(10.0, 20.0), Point::new(90.0, 60.0));
        let scaled = g.scale(new_limits.clone()).unwrap();
        assert_eq!(scaled.data()[0].data()[0], Point::new(50.0, 40.0));
        assert_eq!(scaled.limits().unwrap(), new_limits);
    }

    #[test]
    fn scale_maps_explicit_limits_to_new_limits() {
        let new_limits = Limits::new(Point::new(10.0, 10.0), Point::new(110.0, 210.0));
        let scaled = graph_with_data()
            .with_x_limits(0, 10)
            .with_y_limits(0, 20)
            .scale(new_limits)
            .unwrap();
        let points = scaled.data()[0].data();
        assert_eq!(points[0], Point::new(10.0, 10.0));
        assert_eq!(points[1], Point::new(110.0, 210.0));
    }

    #[test]
    fn scale_with_limits_excluding_a_whole_series() {
        let g = Graph::new()
            .with_series(Series::new(&[Point::new(0.0, 0.0), Point::new(1.0, 1.0)]))
            .with_series(
                Series::new(&[Point::new(50.0, 50.0), Point::new(60.0, 60.0)])
                    .with_line_style(crate::plotting::line::LineStyle::default()),
            )
            .with_x_limits(0.0, 2.0);
        let new_limits = Limits::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        let scaled = g.scale(new_limits).unwrap();
        assert_eq!(scaled.data()[0].data().len(), 2);
        assert!(scaled.data()[1].data().is_empty());
        assert!(scaled.get_mask().is_ok());
    }

    #[test]
    fn scale_with_limits_excluding_all_points_returns_error() {
        let g = graph_with_data().with_x_limits(100, 200);
        let new_limits = Limits::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        let err = g.scale(new_limits).unwrap_err();
        assert!(
            err.to_string().contains("No data points lie within"),
            "{err}"
        );
    }
}
