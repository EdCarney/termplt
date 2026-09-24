use super::{
    axes::{Axes, AxesPositioning},
    colors,
    common::{Drawable, FloatConvertable, MaskPoints},
    graph::Graph,
    limits::Limits,
    point::Point,
    text::{Label, Text, TextPositioning, TextStyle},
    ticks::{AxisTicks, fit_ticks},
};
use crate::{Error, common::Result};
use rgb::RGB8;

#[derive(Debug)]
struct CanvasBuffer {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

/// Empty space left around the edges of the canvas, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferType {
    /// No space.
    None,
    /// The same space on every side.
    Uniform(u32),
    /// Space at the top and bottom.
    TopBottom(u32, u32),
    /// Space on the left and right.
    LeftRight(u32, u32),
    /// Space on each side: top, bottom, left, right.
    TopBottomLeftRight(u32, u32, u32, u32),
}

impl CanvasBuffer {
    pub fn new(buffer_type: BufferType) -> CanvasBuffer {
        match buffer_type {
            BufferType::None => CanvasBuffer::new(BufferType::Uniform(0)),
            BufferType::Uniform(x) => CanvasBuffer {
                left: x,
                top: x,
                right: x,
                bottom: x,
            },
            BufferType::TopBottom(top, bottom) => CanvasBuffer {
                left: 0,
                top,
                right: 0,
                bottom,
            },
            BufferType::LeftRight(left, right) => CanvasBuffer {
                left,
                top: 0,
                right,
                bottom: 0,
            },
            BufferType::TopBottomLeftRight(top, bottom, left, right) => CanvasBuffer {
                left,
                top,
                right,
                bottom,
            },
        }
    }
}

/// An RGB8 pixel buffer, stored row-major from the top row, addressed with (0, 0) at the
/// lower-left corner.
#[derive(Debug)]
pub(crate) struct Canvas {
    bytes: Vec<u8>,
    width: u32,
    height: u32,
}

impl Canvas {
    pub fn new(width: u32, height: u32, background: RGB8) -> Canvas {
        let pixels = width as usize * height as usize;
        let mut bytes = Vec::with_capacity(pixels * 3);
        for _ in 0..pixels {
            bytes.extend_from_slice(&[background.r, background.g, background.b]);
        }
        Canvas {
            bytes,
            width,
            height,
        }
    }

    fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Sets the color of the pixel at (`x`, `y`), with (0, 0) at the lower-left corner. Points
    /// outside the canvas are ignored.
    #[inline]
    pub fn put(&mut self, x: u32, y: u32, color: RGB8) {
        if x < self.width && y < self.height {
            // rows are stored from the top
            let row = (self.height - 1 - y) as usize;
            let i = (row * self.width as usize + x as usize) * 3;
            self.bytes[i..i + 3].copy_from_slice(&[color.r, color.g, color.b]);
        }
    }

    /// Sets the color for multiple points in the canvas.
    pub fn set_pixels(&mut self, points: &[Point<u32>], color: &RGB8) {
        for point in points {
            self.put(point.x, point.y, *color);
        }
    }

    pub fn get_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// Gap in pixels between tick labels and the plot area.
const LABEL_GAP: u32 = 4;
/// Minimum horizontal gap in pixels between neighbouring x tick labels.
const X_LABEL_SPACING: f64 = 8.0;
/// Minimum vertical gap in pixels between neighbouring y tick labels.
const Y_LABEL_SPACING: f64 = 4.0;

/// Where a graph is drawn on the canvas.
struct Layout {
    /// Area the view limits of the data are mapped onto.
    plot: Limits<u32>,
    x_ticks: AxisTicks,
    y_ticks: AxisTicks,
    labels: Vec<Label>,
}

/// A pixel canvas that a [`Graph`] and [`Label`]s are drawn on. [`Plot`](crate::Plot) builds
/// one for you; use it directly for full control over the layout.
#[derive(Debug)]
pub struct TerminalCanvas {
    canvas: Canvas,
    background: RGB8,
    buffer: CanvasBuffer,
    graph: Option<Graph>,
    labels: Vec<Label>,
    limits: Limits<u32>,
}

impl TerminalCanvas {
    /// Creates a canvas of `width` x `height` pixels filled with `background`.
    pub fn new(width: u32, height: u32, background: RGB8) -> TerminalCanvas {
        TerminalCanvas {
            canvas: Canvas::new(width, height, background),
            background,
            buffer: CanvasBuffer::new(BufferType::None),
            graph: None,
            labels: Vec::new(),
            limits: Limits::new(
                Point::new(0, 0),
                Point::new(width.saturating_sub(1), height.saturating_sub(1)),
            ),
        }
    }

    /// Sets the empty space around the edges of the canvas. Tick labels are placed inside the
    /// buffered area automatically, so the buffer does not need to leave room for them.
    pub fn with_buffer(mut self, buffer_type: BufferType) -> Self {
        self.buffer = CanvasBuffer::new(buffer_type);
        self
    }

    /// Sets the graph to draw.
    pub fn with_graph(mut self, graph: Graph) -> Self {
        self.graph = Some(graph);
        self
    }

    /// Adds a text label, drawn after (over) the graph.
    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    /// The canvas pixels as RGB8 bytes, row-major from the top row.
    pub fn get_bytes(&self) -> Vec<u8> {
        self.canvas.get_bytes()
    }

    /// Like [`TerminalCanvas::get_bytes`], without copying.
    pub fn into_bytes(self) -> Vec<u8> {
        self.canvas.into_bytes()
    }

    /// Consumes all drawable assets and draws them on the canvas.
    pub fn draw(mut self) -> Result<Self> {
        if self.canvas.is_empty() {
            return Err(Error::CanvasTooSmall {
                plot_width: 0,
                plot_height: 0,
            });
        }

        if let Some(graph) = self.graph.take() {
            let view = graph.view_limits()?;
            let layout = self.layout(&graph, &view)?;
            let plot = layout.plot.convert_to_f64();
            let scaled_graph = graph.scale_with_view(&view, plot.clone())?;

            let mut masks = Vec::new();

            // grid lines first so the axes and data are drawn over them
            if let Some(grid_lines) = graph.grid_lines() {
                let xs: Vec<f64> = (layout.x_ticks.values.iter())
                    .map(|&v| to_canvas(v, view.min().x, view.max().x, plot.min().x, plot.max().x))
                    .collect();
                let ys: Vec<f64> = (layout.y_ticks.values.iter())
                    .map(|&v| to_canvas(v, view.min().y, view.max().y, plot.min().y, plot.max().y))
                    .collect();
                masks.extend(grid_lines.get_mask_at(&plot, &xs, &ys)?);
            }
            if let Some(axes) = graph.axes() {
                masks.extend(axes.get_mask(&plot)?);
            }
            masks
                .iter()
                .for_each(|mask| self.canvas.set_pixels(&mask.points, &mask.color));
            // series are drawn straight into the canvas: they can have millions of pixels
            for series in scaled_graph.data() {
                series.draw_into(&mut self.canvas)?;
            }
            self.labels.extend(layout.labels);
        }

        // labels are drawn last so they are not covered by the graph
        let label_masks: Vec<Vec<MaskPoints>> = self
            .labels
            .iter()
            .map(|txt| txt.get_mask())
            .collect::<Result<_>>()?;
        label_masks
            .iter()
            .flatten()
            .for_each(|mask| self.canvas.set_pixels(&mask.points, &mask.color));

        Ok(self)
    }

    /// Returns the area the graph's data is mapped onto: the canvas minus the buffer, the space
    /// taken by tick labels, and an inset for markers, thick lines and axes.
    pub fn get_drawable_limits(&self) -> Result<Limits<u32>> {
        match &self.graph {
            Some(graph) => Ok(self.layout(graph, &graph.view_limits()?)?.plot),
            None => {
                let (min, max) = self.buffered_area();
                check_area(&min, &max)?;
                Ok(Limits::new(min, max))
            }
        }
    }

    /// Corners of the canvas inside the buffer. Uses saturating arithmetic so a canvas smaller
    /// than its buffer yields an empty area rather than overflowing.
    fn buffered_area(&self) -> (Point<u32>, Point<u32>) {
        let min = Point::new(self.buffer.left, self.buffer.bottom);
        let max = Point::new(
            self.limits.max().x.saturating_sub(self.buffer.right),
            self.limits.max().y.saturating_sub(self.buffer.top),
        );
        (min, max)
    }

    /// Text style for tick labels. A label color identical to the background (e.g. the black
    /// default text on the default black canvas) is replaced with black or white, whichever
    /// contrasts with the background.
    fn label_style(&self, axes: Option<&Axes>) -> TextStyle {
        let style = axes.map(|a| a.style().clone()).unwrap_or_default();
        let color = if style.color() == self.background {
            let luminance = 0.2126 * self.background.r as f64
                + 0.7152 * self.background.g as f64
                + 0.0722 * self.background.b as f64;
            if luminance > 127.5 {
                colors::BLACK
            } else {
                colors::WHITE
            }
        } else {
            style.color()
        };
        TextStyle::new(color, style.scale(), style.padding())
    }

    fn layout(&self, graph: &Graph, view: &Limits<f64>) -> Result<Layout> {
        let (outer_min, outer_max) = self.buffered_area();

        let largest_marker_sz = graph
            .data()
            .iter()
            // thick lines extend past the data points just like markers do
            .map(|s| {
                let line_thickness = s.line_style().map_or(0, |l| l.thickness());
                s.marker_style().size().max(line_thickness)
            })
            .max()
            .ok_or(Error::NoData)?;

        // axes are drawn just outside the plot area, so leave room for their thickness
        let axes = graph.axes().cloned();
        let (axes_inset, show_x_labels, show_y_labels) =
            match axes.as_ref().map(|a| a.positioning()) {
                Some(AxesPositioning::XOnly(line)) => ((0, 2 * line.thickness()), true, false),
                Some(AxesPositioning::YOnly(line)) => ((2 * line.thickness(), 0), false, true),
                Some(AxesPositioning::XY(line)) => {
                    ((2 * line.thickness(), 2 * line.thickness()), true, true)
                }
                None => ((0, 0), false, false),
            };
        // axes and markers can overlap, so use the larger of the two
        let inset_x = largest_marker_sz.max(axes_inset.0);
        let inset_y = largest_marker_sz.max(axes_inset.1);

        let style = self.label_style(axes.as_ref());
        let text = |label: &str| Text::new(label, style.clone());
        let text_h = text("0").height() as u32;

        // x labels sit in a band along the bottom; the top y label needs half a line above
        let bottom = if show_x_labels { text_h + LABEL_GAP } else { 0 };
        let top = if show_y_labels { text_h / 2 } else { 0 };
        let plot_min_y = outer_min.y + bottom + inset_y;
        let plot_max_y = outer_max.y.saturating_sub(top + inset_y);
        check_area(
            &Point::new(outer_min.x, plot_min_y),
            &Point::new(outer_max.x, plot_max_y),
        )?;

        // y ticks depend only on the plot height; their labels then set the left margin
        let y_ticks = fit_ticks(
            view.min().y,
            view.max().y,
            (plot_max_y - plot_min_y) as f64,
            |_, spacing| spacing >= text_h as f64 + Y_LABEL_SPACING,
        );
        let y_label_w = if show_y_labels {
            (y_ticks.labels.iter())
                .map(|l| text(l).width() as u32)
                .max()
                .unwrap_or(0)
        } else {
            0
        };
        let left = if show_y_labels {
            y_label_w + LABEL_GAP
        } else {
            0
        };
        let plot_min_x = outer_min.x + left + inset_x;
        let plot_max_x = outer_max.x.saturating_sub(inset_x);
        let plot_min = Point::new(plot_min_x, plot_min_y);
        let plot_max = Point::new(plot_max_x, plot_max_y);
        check_area(&plot_min, &plot_max)?;

        let x_ticks = fit_ticks(
            view.min().x,
            view.max().x,
            (plot_max_x - plot_min_x) as f64,
            |labels, spacing| {
                let widest = labels.iter().map(|l| text(l).width()).max().unwrap_or(0);
                spacing >= widest as f64 + X_LABEL_SPACING
            },
        );

        let plot = Limits::new(plot_min, plot_max);
        let (canvas_max_x, canvas_max_y) = (self.limits.max().x, self.limits.max().y);
        let mut labels = Vec::new();

        if show_x_labels {
            let center_y = outer_min.y + text_h / 2;
            for (&value, label) in x_ticks.values.iter().zip(&x_ticks.labels) {
                let txt = text(label);
                let w = txt.width() as u32;
                let x = to_canvas(
                    value,
                    view.min().x,
                    view.max().x,
                    plot_min_x as f64,
                    plot_max_x as f64,
                );
                // keep labels at the ends of the axis inside the canvas
                let lo = w / 2;
                let hi = canvas_max_x.saturating_sub(w - w / 2).max(lo);
                let x = (x.round() as u32).clamp(lo, hi);
                labels.push(Label::new(
                    txt,
                    TextPositioning::Centered(Point::new(x, center_y)),
                ));
            }
        }

        if show_y_labels {
            // stay above the x label band and inside the canvas
            let lo = if show_x_labels {
                outer_min.y + text_h + 1 + text_h / 2
            } else {
                text_h / 2
            };
            let hi = canvas_max_y.saturating_sub(text_h - text_h / 2).max(lo);
            for (&value, label) in y_ticks.values.iter().zip(&y_ticks.labels) {
                let txt = text(label);
                // right-align labels against the plot area
                let x = outer_min.x + y_label_w - txt.width() as u32;
                let y = to_canvas(
                    value,
                    view.min().y,
                    view.max().y,
                    plot_min_y as f64,
                    plot_max_y as f64,
                );
                let y = (y.round() as u32).clamp(lo, hi);
                labels.push(Label::new(
                    txt,
                    TextPositioning::LeftAligned(Point::new(x, y)),
                ));
            }
        }

        Ok(Layout {
            plot,
            x_ticks,
            y_ticks,
            labels,
        })
    }
}

/// Linearly maps `value` from the data range `[view_min, view_max]` onto the pixel range
/// `[plot_min, plot_max]`.
fn to_canvas(value: f64, view_min: f64, view_max: f64, plot_min: f64, plot_max: f64) -> f64 {
    plot_min + (value - view_min) * (plot_max - plot_min) / (view_max - view_min)
}

fn check_area(min: &Point<u32>, max: &Point<u32>) -> Result<()> {
    if min.x >= max.x || min.y >= max.y {
        return Err(Error::CanvasTooSmall {
            plot_width: max.x.saturating_sub(min.x),
            plot_height: max.y.saturating_sub(min.y),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plotting::{colors, series::Series};

    #[test]
    fn empty_graph_returns_error() {
        let result = TerminalCanvas::new(100, 100, colors::BLACK)
            .with_graph(Graph::new())
            .draw();
        assert!(result.is_err());
    }

    #[test]
    fn zero_sized_canvas_returns_error() {
        let points = [Point::new(0.0, 0.0), Point::new(1.0, 1.0)];
        for (w, h) in [(0, 10), (10, 0), (0, 0)] {
            let result = TerminalCanvas::new(w, h, colors::BLACK)
                .with_graph(Graph::new().with_series(Series::new(&points)))
                .draw();
            assert!(result.is_err(), "{w}x{h} canvas should error");
        }
    }

    #[test]
    fn single_series() {
        let points = (0..=5).map(|x| Point::new(x, x)).collect::<Vec<Point<_>>>();
        TerminalCanvas::new(100, 100, colors::BLACK)
            .with_buffer(BufferType::Uniform(5))
            .with_graph(Graph::new().with_series(Series::new(&points)))
            .draw()
            .unwrap();
    }

    #[test]
    fn canvas_too_small_for_buffer_returns_error() {
        let points = (0..=5).map(|x| Point::new(x, x)).collect::<Vec<Point<_>>>();
        let result = TerminalCanvas::new(50, 50, colors::BLACK)
            .with_buffer(BufferType::Uniform(30))
            .with_graph(Graph::new().with_series(Series::new(&points)))
            .draw();
        assert!(matches!(result, Err(Error::CanvasTooSmall { .. })));
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("canvas is too small")
        );
    }

    #[test]
    fn buffer_type_top_bottom() {
        let points = (0..=5).map(|x| Point::new(x, x)).collect::<Vec<Point<_>>>();
        TerminalCanvas::new(100, 100, colors::BLACK)
            .with_buffer(BufferType::TopBottom(10, 5))
            .with_graph(Graph::new().with_series(Series::new(&points)))
            .draw()
            .unwrap();
    }

    #[test]
    fn buffer_type_left_right() {
        let points = (0..=5).map(|x| Point::new(x, x)).collect::<Vec<Point<_>>>();
        TerminalCanvas::new(100, 100, colors::BLACK)
            .with_buffer(BufferType::LeftRight(10, 5))
            .with_graph(Graph::new().with_series(Series::new(&points)))
            .draw()
            .unwrap();
    }

    #[test]
    fn buffer_type_top_bottom_left_right() {
        let points = (0..=5).map(|x| Point::new(x, x)).collect::<Vec<Point<_>>>();
        TerminalCanvas::new(100, 100, colors::BLACK)
            .with_buffer(BufferType::TopBottomLeftRight(10, 5, 8, 3))
            .with_graph(Graph::new().with_series(Series::new(&points)))
            .draw()
            .unwrap();
    }

    #[test]
    fn buffer_type_top_bottom_left_right_fields_are_correct() {
        let buf = CanvasBuffer::new(BufferType::TopBottomLeftRight(10, 5, 8, 3));
        assert_eq!(buf.top, 10);
        assert_eq!(buf.bottom, 5);
        assert_eq!(buf.left, 8);
        assert_eq!(buf.right, 3);
    }

    #[test]
    fn buffer_type_top_bottom_fields_are_correct() {
        let buf = CanvasBuffer::new(BufferType::TopBottom(10, 5));
        assert_eq!(buf.top, 10);
        assert_eq!(buf.bottom, 5);
        assert_eq!(buf.left, 0);
        assert_eq!(buf.right, 0);
    }

    #[test]
    fn buffer_type_left_right_fields_are_correct() {
        let buf = CanvasBuffer::new(BufferType::LeftRight(8, 3));
        assert_eq!(buf.left, 8);
        assert_eq!(buf.right, 3);
        assert_eq!(buf.top, 0);
        assert_eq!(buf.bottom, 0);
    }

    fn red_pixels(bytes: &[u8], width: usize) -> Vec<(usize, usize)> {
        bytes
            .chunks(3)
            .enumerate()
            .filter(|(_, px)| *px == [255, 0, 0])
            .map(|(i, _)| (i % width, i / width))
            .collect()
    }

    fn red_marker() -> crate::plotting::marker::MarkerStyle {
        crate::plotting::marker::MarkerStyle::FilledSquare {
            size: 0,
            color: colors::RED,
        }
    }

    #[test]
    fn single_point_is_drawn_at_center() {
        let bytes =
            TerminalCanvas::new(101, 101, colors::BLACK)
                .with_buffer(BufferType::Uniform(10))
                .with_graph(Graph::new().with_series(
                    Series::new(&[Point::new(5.0, 5.0)]).with_marker_style(red_marker()),
                ))
                .draw()
                .unwrap()
                .get_bytes();
        assert_eq!(red_pixels(&bytes, 101), vec![(50, 50)]);
    }

    #[test]
    fn constant_series_is_drawn_at_vertical_center() {
        let points = [Point::new(0.0, 5.0), Point::new(10.0, 5.0)];
        let bytes = TerminalCanvas::new(101, 101, colors::BLACK)
            .with_buffer(BufferType::Uniform(10))
            .with_graph(
                Graph::new().with_series(Series::new(&points).with_marker_style(red_marker())),
            )
            .draw()
            .unwrap()
            .get_bytes();
        // x is padded by 5% of the span on each side: 10 + 80 * 0.5 / 11 = 13.6 (truncated)
        assert_eq!(red_pixels(&bytes, 101), vec![(13, 50), (86, 50)]);
    }

    #[test]
    fn non_finite_points_with_axes_draw() {
        use crate::plotting::{
            axes::{Axes, AxesPositioning},
            line::LineStyle,
            text::TextStyle,
        };
        let points = [
            Point::new(f64::NAN, 0.0),
            Point::new(1.0, 1.0),
            Point::new(f64::INFINITY, 2.0),
            Point::new(2.0, 3.0),
        ];
        TerminalCanvas::new(200, 200, colors::BLACK)
            .with_buffer(BufferType::Uniform(40))
            .with_graph(
                Graph::new()
                    .with_series(Series::new(&points))
                    .with_axes(Axes::new(
                        AxesPositioning::XY(LineStyle::default()),
                        TextStyle::default(),
                    )),
            )
            .draw()
            .unwrap();
    }

    #[test]
    fn inverted_limits_return_error() {
        let points = [Point::new(0.0, 0.0), Point::new(1.0, 1.0)];
        let result = TerminalCanvas::new(200, 200, colors::BLACK)
            .with_graph(
                Graph::new()
                    .with_series(Series::new(&points))
                    .with_x_limits(2.0, 0.0),
            )
            .draw();
        assert!(result.is_err());
    }

    #[test]
    fn tiny_value_single_point_is_centered() {
        // a single point at the smallest normal f64 has a subnormal padded span; scaling must
        // not overflow (found by property testing)
        let bytes = TerminalCanvas::new(101, 101, colors::BLACK)
            .with_buffer(BufferType::Uniform(10))
            .with_graph(Graph::new().with_series(
                Series::new(&[Point::new(f64::MIN_POSITIVE, 0.0)]).with_marker_style(red_marker()),
            ))
            .draw()
            .unwrap()
            .get_bytes();
        assert_eq!(red_pixels(&bytes, 101), vec![(50, 50)]);
    }

    #[test]
    fn data_range_exceeding_f64_returns_error() {
        let points = [Point::new(-f64::MAX, 0.0), Point::new(f64::MAX, 1.0)];
        let result = TerminalCanvas::new(100, 100, colors::BLACK)
            .with_graph(Graph::new().with_series(Series::new(&points)))
            .draw();
        assert!(result.is_err());
    }
}
