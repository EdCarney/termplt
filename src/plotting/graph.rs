use super::{
    axes::Axes,
    common::{Drawable, Graphable, MaskPoints},
    grid_lines::GridLines,
    limits::Limits,
    point::Point,
    series::Series,
};
use crate::{Error, common::Result};

/// A set of series drawn on shared axes, with optional axis limits, axes and grid lines.
#[derive(Debug, Clone, Default)]
pub struct Graph {
    data: Vec<Series>,
    x_limits: Option<(f64, f64)>,
    y_limits: Option<(f64, f64)>,
    axes: Option<Axes>,
    grid_lines: Option<GridLines>,
}

impl Graph {
    /// Creates an empty graph.
    pub fn new() -> Graph {
        Graph::default()
    }

    /// Adds a series; series are drawn in the order they are added.
    pub fn with_series(mut self, series: Series) -> Self {
        self.data.push(series);
        self
    }

    /// Draws axes (with tick labels) around the plot area.
    pub fn with_axes(mut self, axes: Axes) -> Self {
        self.axes = Some(axes);
        self
    }

    /// Draws grid lines at the tick positions.
    pub fn with_grid_lines(mut self, grid_lines: GridLines) -> Self {
        self.grid_lines = Some(grid_lines);
        self
    }

    /// Fixes the x range; points outside it are not drawn. Without it, the range fits the data.
    pub fn with_x_limits<T: Graphable>(mut self, min: T, max: T) -> Self {
        self.x_limits = Some((min.to_f64(), max.to_f64()));
        self
    }

    /// Fixes the y range; points outside it are not drawn. Without it, the range fits the data.
    pub fn with_y_limits<T: Graphable>(mut self, min: T, max: T) -> Self {
        self.y_limits = Some((min.to_f64(), max.to_f64()));
        self
    }

    /// The series, in drawing order.
    pub fn data(&self) -> &[Series] {
        &self.data
    }

    /// The axes, if any.
    pub fn axes(&self) -> Option<&Axes> {
        self.axes.as_ref()
    }

    /// The grid lines, if any.
    pub fn grid_lines(&self) -> Option<&GridLines> {
        self.grid_lines.as_ref()
    }

    /// The explicit x range, if one was set.
    pub fn x_limits(&self) -> Option<(f64, f64)> {
        self.x_limits
    }

    /// The explicit y range, if one was set.
    pub fn y_limits(&self) -> Option<(f64, f64)> {
        self.y_limits
    }

    /// Returns the plotted range: the extent of all finite data points, overridden by any
    /// explicit limits. Non-finite points (NaN, ±∞) are ignored.
    ///
    /// Errors if the graph has no finite data points or the explicit limits are invalid
    /// (inverted or non-finite).
    pub fn limits(&self) -> Result<Limits<f64>> {
        self.validate_limits()?;

        // one pass, without collecting: graphs can have millions of points
        let data_limits = self
            .data
            .iter()
            .flat_map(|series| series.data().iter().copied())
            .filter(is_finite_point)
            .fold(None, |acc: Option<(Point<f64>, Point<f64>)>, p| {
                Some(match acc {
                    None => (p, p),
                    Some((min, max)) => (
                        Point::new(min.x.min(p.x), min.y.min(p.y)),
                        Point::new(max.x.max(p.x), max.y.max(p.y)),
                    ),
                })
            })
            .map(|(min, max)| Limits::new(min, max))
            .ok_or(Error::NoData)?;

        // explicit limits override data limits
        let (x_min, x_max) = self
            .x_limits
            .unwrap_or((data_limits.min().x, data_limits.max().x));
        let (y_min, y_max) = self
            .y_limits
            .unwrap_or((data_limits.min().y, data_limits.max().y));
        Ok(Limits::new(
            Point::new(x_min, y_min),
            Point::new(x_max, y_max),
        ))
    }

    fn validate_limits(&self) -> Result<()> {
        let check = |axis: &'static str, (min, max): (f64, f64)| -> Result<()> {
            if !min.is_finite() || !max.is_finite() || min > max {
                return Err(Error::InvalidLimits { axis, min, max });
            }
            Ok(())
        };
        self.x_limits.map_or(Ok(()), |l| check("x", l))?;
        self.y_limits.map_or(Ok(()), |l| check("y", l))
    }

    fn has_explicit_limits(&self) -> bool {
        self.x_limits.is_some() || self.y_limits.is_some()
    }

    /// Returns a copy of the graph containing only the points that will be drawn: finite points
    /// that lie within the explicit limits (if any). Series left empty are kept.
    fn visible(&self) -> Result<Graph> {
        let limits = self.limits()?;
        let clip = self.has_explicit_limits();
        let data = self
            .data
            .iter()
            .map(|series| {
                series.map_points(|points| {
                    points
                        .iter()
                        .copied()
                        .filter(|p| is_finite_point(p) && (!clip || limits.contains(p)))
                        .collect()
                })
            })
            .collect();

        Ok(Graph {
            data,
            ..self.clone_without_data()
        })
    }

    fn clone_without_data(&self) -> Graph {
        Graph {
            data: Vec::new(),
            x_limits: self.x_limits,
            y_limits: self.y_limits,
            axes: self.axes.clone(),
            grid_lines: self.grid_lines.clone(),
        }
    }

    /// Returns the data range that will be mapped onto the drawable area: [`Graph::limits`]
    /// after clipping to explicit limits, with a margin of [`DATA_MARGIN`] of the span added on
    /// axes without explicit limits (so data does not touch the axes), and zero-width
    /// dimensions (e.g. a single point or a constant series) expanded so the data is centered.
    pub fn view_limits(&self) -> Result<Limits<f64>> {
        let limits = self.visible()?.limits().map_err(|_| Error::NoVisibleData)?;

        let (span_x, span_y) = limits.span();
        let margin = Point::new(
            if self.x_limits.is_some() {
                0.0
            } else {
                span_x * DATA_MARGIN
            },
            if self.y_limits.is_some() {
                0.0
            } else {
                span_y * DATA_MARGIN
            },
        );
        let limits = pad_degenerate(Limits::new(*limits.min() - margin, *limits.max() + margin));

        let (span_x, span_y) = limits.span();
        if !span_x.is_finite() || !span_y.is_finite() {
            return Err(Error::DataRangeTooLarge);
        }
        Ok(limits)
    }

    /// Scales the visible data so that [`Graph::view_limits`] maps onto `new_limits`. The
    /// returned graph's explicit limits are `new_limits`.
    pub fn scale(self, new_limits: Limits<f64>) -> Result<Graph> {
        let view_limits = self.view_limits()?;
        self.scale_with_view(&view_limits, new_limits)
    }

    pub(crate) fn scale_with_view(
        &self,
        view_limits: &Limits<f64>,
        new_limits: Limits<f64>,
    ) -> Result<Graph> {
        let (old_min, new_min) = (*view_limits.min(), *new_limits.min());
        let (old_span_x, old_span_y) = view_limits.span();
        let (new_span_x, new_span_y) = new_limits.span();
        // divide first so extreme spans cannot overflow; a zero span maps to the middle
        let map = |v: f64, old_min: f64, old_span: f64, new_min: f64, new_span: f64| {
            if old_span == 0.0 {
                new_min + new_span / 2.0
            } else {
                new_min + (v - old_min) / old_span * new_span
            }
        };
        let scale = |p: Point<f64>| {
            Point::new(
                map(p.x, old_min.x, old_span_x, new_min.x, new_span_x),
                map(p.y, old_min.y, old_span_y, new_min.y, new_span_y),
            )
        };

        let data = self
            .visible()?
            .data
            .iter()
            .map(|series| series.map_points(|points| points.iter().map(|&p| scale(p)).collect()))
            .collect();

        // the view maps exactly onto the new limits; record them explicitly so the axes and grid
        // span the whole drawable area even where the data does not
        Ok(Graph {
            data,
            x_limits: Some((new_limits.min().x, new_limits.max().x)),
            y_limits: Some((new_limits.min().y, new_limits.max().y)),
            ..self.clone_without_data()
        })
    }
}

/// Fraction of the data span added on each side of axes without explicit limits.
pub const DATA_MARGIN: f64 = 0.05;

fn is_finite_point(p: &Point<f64>) -> bool {
    p.x.is_finite() && p.y.is_finite()
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

impl Drawable for Graph {
    /// Draws the graph in its own coordinates, which must already be pixel coordinates (see
    /// [`Graph::scale`]).
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

        for series in &self.data {
            mask_points.extend(series.get_mask()?);
        }

        Ok(mask_points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph() {
        let g = Graph::new();
        assert!(g.limits().is_err());
    }

    #[test]
    fn add_single_series_with_single_point() {
        let g = Graph::new().with_series(Series::new(&[Point::new(0, 0)]));

        let limits = g.limits();
        assert!(limits.is_ok());
        assert_eq!(
            limits.unwrap(),
            Limits::new(Point::new(0.0, 0.0), Point::new(0.0, 0.0))
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
            Limits::new(Point::new(-1.0, -5.0), Point::new(10.0, 15.0))
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
            Limits::new(Point::new(-1.0, -5.0), Point::new(10.0, 15.0))
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
            Limits::new(Point::new(-20.0, -50.0), Point::new(100.0, 50.0))
        );
    }

    // --- with_x_limits state machine ---

    fn graph_with_data() -> Graph {
        Graph::new().with_series(Series::new(&[Point::new(0, 0), Point::new(10, 20)]))
    }

    #[test]
    fn with_x_limits_from_none_produces_x_only() {
        // None → XOnly: x limits override data, y limits come from data
        let g = graph_with_data().with_x_limits(-5, 15);
        let limits = g.limits().unwrap();
        assert_eq!(limits.min().x, -5.0);
        assert_eq!(limits.max().x, 15.0);
        // y should still come from data
        assert_eq!(limits.min().y, 0.0);
        assert_eq!(limits.max().y, 20.0);
    }

    #[test]
    fn with_x_limits_from_x_only_replaces_x() {
        // XOnly → XOnly: new x limits replace old x limits
        let g = graph_with_data()
            .with_x_limits(-5, 15)
            .with_x_limits(-100, 100);
        let limits = g.limits().unwrap();
        assert_eq!(limits.min().x, -100.0);
        assert_eq!(limits.max().x, 100.0);
        assert_eq!(limits.min().y, 0.0);
        assert_eq!(limits.max().y, 20.0);
    }

    #[test]
    fn with_x_limits_from_y_only_produces_xy() {
        // YOnly → XY: combining x and y limits
        let g = graph_with_data()
            .with_y_limits(-10, 30)
            .with_x_limits(-5, 15);
        let limits = g.limits().unwrap();
        assert_eq!(limits.min().x, -5.0);
        assert_eq!(limits.max().x, 15.0);
        assert_eq!(limits.min().y, -10.0);
        assert_eq!(limits.max().y, 30.0);
    }

    #[test]
    fn with_x_limits_from_xy_updates_only_x() {
        // XY → XY: only x changes
        let g = graph_with_data()
            .with_x_limits(-5, 15)
            .with_y_limits(-10, 30)
            .with_x_limits(-50, 50);
        let limits = g.limits().unwrap();
        assert_eq!(limits.min().x, -50.0);
        assert_eq!(limits.max().x, 50.0);
        // y should be preserved from the earlier with_y_limits call
        assert_eq!(limits.min().y, -10.0);
        assert_eq!(limits.max().y, 30.0);
    }

    // --- with_y_limits state machine ---

    #[test]
    fn with_y_limits_from_none_produces_y_only() {
        // None → YOnly: y limits override data, x limits come from data
        let g = graph_with_data().with_y_limits(-10, 30);
        let limits = g.limits().unwrap();
        assert_eq!(limits.min().x, 0.0);
        assert_eq!(limits.max().x, 10.0);
        assert_eq!(limits.min().y, -10.0);
        assert_eq!(limits.max().y, 30.0);
    }

    #[test]
    fn with_y_limits_from_y_only_replaces_y() {
        // YOnly → YOnly: new y limits replace old y limits
        let g = graph_with_data()
            .with_y_limits(-10, 30)
            .with_y_limits(-100, 100);
        let limits = g.limits().unwrap();
        assert_eq!(limits.min().x, 0.0);
        assert_eq!(limits.max().x, 10.0);
        assert_eq!(limits.min().y, -100.0);
        assert_eq!(limits.max().y, 100.0);
    }

    #[test]
    fn with_y_limits_from_x_only_produces_xy() {
        // XOnly → XY: combining x and y limits
        let g = graph_with_data()
            .with_x_limits(-5, 15)
            .with_y_limits(-10, 30);
        let limits = g.limits().unwrap();
        assert_eq!(limits.min().x, -5.0);
        assert_eq!(limits.max().x, 15.0);
        assert_eq!(limits.min().y, -10.0);
        assert_eq!(limits.max().y, 30.0);
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
        assert_eq!(limits.min().x, -5.0);
        assert_eq!(limits.max().x, 15.0);
        assert_eq!(limits.min().y, -50.0);
        assert_eq!(limits.max().y, 50.0);
    }

    // --- limits() with no data is an error even with graph_limits ---

    #[test]
    fn limits_with_no_data_returns_error() {
        let g = Graph::new().with_x_limits(0, 10);
        assert!(
            g.limits().is_err(),
            "No data means no limits, even with explicit graph limits"
        );
    }

    #[test]
    fn get_mask_on_empty_graph_returns_error() {
        let g = Graph::new();
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
        assert!(matches!(err, Error::NoVisibleData), "{err}");
    }
}
