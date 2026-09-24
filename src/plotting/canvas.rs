use super::{
    axes::AxesPositioning,
    common::{Drawable, FloatConvertable, Graphable, MaskPoints},
    graph::Graph,
    limits::Limits,
    point::Point,
    text::Label,
};
use crate::common::Result;
use rgb::RGB8;

#[derive(Debug)]
struct CanvasBuffer {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

pub enum BufferType {
    None,
    Uniform(u32),
    TopBottom(u32, u32),
    LeftRight(u32, u32),
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

#[derive(Debug)]
struct Canvas {
    pixels: Vec<Vec<RGB8>>,
    limits: Limits<u32>,
}

impl Canvas {
    pub fn new(width: u32, height: u32, background: RGB8) -> Canvas {
        Canvas {
            pixels: (0..height)
                .map(|_| vec![background; width as usize])
                .collect(),
            limits: Limits::new(
                Point::new(0, 0),
                Point::new(width.saturating_sub(1), height.saturating_sub(1)),
            ),
        }
    }

    /// Sets the color for a point in the canvas. The provided point should be zero-indexed with
    /// the lower-left corner as (0, 0) and the upper-right cornder as (width - 1, height - 1).
    pub fn set_pixel(&mut self, point: &Point<u32>, color: &RGB8) {
        if self.limits.contains(point) {
            // reverse y since higher values means closer to
            // the top of the canvas
            let x = point.x as usize;
            let y = (self.limits.max().y - point.y) as usize;
            // bounds-checked since a zero-sized canvas still has 0..=0 limits
            if let Some(pixel) = self.pixels.get_mut(y).and_then(|row| row.get_mut(x)) {
                *pixel = *color;
            }
        }
    }

    /// Sets the color for multiple points in the canvas. The provided points should be zero-indexed
    /// with the lower-left corner as (0, 0) and the upper-right cornder as (width - 1, height - 1).
    pub fn set_pixels(&mut self, points: &[Point<u32>], color: &RGB8) {
        for point in points {
            self.set_pixel(point, color);
        }
    }

    pub fn get_bytes(&self) -> Vec<u8> {
        self.pixels
            .iter()
            .flat_map(|row| {
                row.iter()
                    .flat_map(|&rgb| [rgb.r, rgb.g, rgb.b])
                    .collect::<Vec<u8>>()
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct TerminalCanvas<T: Graphable> {
    canvas: Canvas,
    buffer: CanvasBuffer,
    graph: Option<Graph<T>>,
    labels: Vec<Label>,
    limits: Limits<u32>,
}

impl<T> TerminalCanvas<T>
where
    T: Graphable + FloatConvertable,
{
    pub fn new(width: u32, height: u32, background: RGB8) -> TerminalCanvas<T> {
        TerminalCanvas {
            canvas: Canvas::new(width, height, background),
            buffer: CanvasBuffer::new(BufferType::None),
            graph: None,
            labels: Vec::new(),
            limits: Limits::new(
                Point::new(0, 0),
                Point::new(width.saturating_sub(1), height.saturating_sub(1)),
            ),
        }
    }

    pub fn with_buffer(mut self, buffer_type: BufferType) -> Self {
        self.buffer = CanvasBuffer::new(buffer_type);
        self
    }

    pub fn with_graph(mut self, graph: Graph<T>) -> Self {
        self.graph = Some(graph);
        self
    }

    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    pub fn get_bytes(&self) -> Vec<u8> {
        self.canvas.get_bytes()
    }

    /// Consumes all drawable assets and draws them on the canvas.
    pub fn draw(mut self) -> Result<Self> {
        if self.canvas.pixels.is_empty() || self.canvas.pixels[0].is_empty() {
            return Err("Canvas width and height must be nonzero".into());
        }

        let canvas_limits = self.get_drawable_limits()?.convert_to_f64();

        if let Some(graph) = self.graph.take() {
            // the view limits are the data values at the edges of the drawable area, so they
            // determine the numbers shown on the axes labels
            let view_limits = graph.view_limits()?;
            let scaled_graph = graph.scale_with_view(&view_limits, canvas_limits)?;

            scaled_graph
                .get_axes_labels(&view_limits)?
                .into_iter()
                .for_each(|label| self.labels.push(label));

            scaled_graph
                .get_mask()?
                .iter()
                .for_each(|mask| self.canvas.set_pixels(&mask.points, &mask.color));
        }

        // labels must be drawn after graph since axes labels are added to the canvas
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

    pub fn get_drawable_limits(&self) -> Result<Limits<u32>> {
        // set initial point from the buffer; use saturating_sub to avoid u32 overflow
        // when the canvas is smaller than the buffer
        let mut min = Point::new(self.buffer.left, self.buffer.bottom);
        let mut max = Point::new(
            self.limits.max().x.saturating_sub(self.buffer.right),
            self.limits.max().y.saturating_sub(self.buffer.top),
        );

        if let Some(graph) = &self.graph {
            let largest_marker_sz = graph
                .data()
                .iter()
                // thick lines extend past the data points just like markers do
                .map(|s| {
                    let line_thickness = s.line_style().map_or(0, |l| l.thickness());
                    s.marker_style().size().max(line_thickness)
                })
                .max()
                .ok_or("Graph has no series data; cannot compute drawable limits")?;

            // axes thickness in x/y pixels
            let axes_thickness = match graph.axes() {
                Some(axes) => match axes.positioning() {
                    AxesPositioning::XOnly(line_style) => (0, 2 * line_style.thickness()),
                    AxesPositioning::YOnly(line_style) => (2 * line_style.thickness(), 0),
                    AxesPositioning::XY(line_style) => {
                        (2 * line_style.thickness(), 2 * line_style.thickness())
                    }
                },
                None => (0, 0),
            };

            // note that axes and markers can overlap; so use the larger of marker/axes as bounds
            let inset_x = u32::max(largest_marker_sz, axes_thickness.0);
            let inset_y = u32::max(largest_marker_sz, axes_thickness.1);

            let min_x = min.x + inset_x;
            let min_y = min.y + inset_y;
            let max_x = max.x.saturating_sub(inset_x);
            let max_y = max.y.saturating_sub(inset_y);

            // include axes text

            min = Point::new(min_x, min_y);
            max = Point::new(max_x, max_y);
        }

        if min.x >= max.x || min.y >= max.y {
            return Err(format!(
                "Canvas too small for the configured buffer and graph elements. \
                 Drawable area would be {}x{} pixels (min={:?}, max={:?}). \
                 Try a larger terminal window or smaller buffer/marker sizes.",
                max.x.saturating_sub(min.x),
                max.y.saturating_sub(min.y),
                min,
                max,
            )
            .into());
        }

        Ok(Limits::new(min, max))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plotting::{colors, series::Series};

    #[test]
    fn empty_graph_returns_error() {
        let result = TerminalCanvas::new(100, 100, colors::BLACK)
            .with_graph(Graph::<u32>::new())
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
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Canvas too small"));
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
        assert_eq!(red_pixels(&bytes, 101), vec![(10, 50), (90, 50)]);
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
}
