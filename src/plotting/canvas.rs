use super::{
    axes::{Axes, AxesPositioning},
    colors,
    common::FloatConvertable,
    font::{Coverage, Font},
    graph::Graph,
    limits::Limits,
    point::Point,
    srgb,
    text::{DEFAULT_FONT_SIZE, Label, MAX_FONT_SIZE, TextPositioning},
    ticks::{AxisTicks, axis_offset, fit_ticks, format_offset},
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

    /// Draws `color` over the pixel at (`x`, `y`) with `coverage` (0 = none, 255 = all), mixing
    /// in linear light. (0, 0) is the lower-left corner; points outside the canvas are ignored.
    pub fn blend(&mut self, x: u32, y: u32, color: RGB8, coverage: u8) {
        if x < self.width && y < self.height {
            // rows are stored from the top
            let row = (self.height - 1 - y) as usize;
            let i = (row * self.width as usize + x as usize) * 3;
            for (channel, fg) in self.bytes[i..i + 3]
                .iter_mut()
                .zip([color.r, color.g, color.b])
            {
                *channel = srgb::blend(*channel, fg, coverage);
            }
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
    /// Ticks relative to `x_offset`.
    x_ticks: AxisTicks,
    /// Ticks relative to `y_offset`.
    y_ticks: AxisTicks,
    /// Offset of the x axis ([`axis_offset`]), 0 for none.
    x_offset: f64,
    /// Offset of the y axis, 0 for none.
    y_offset: f64,
    /// Tick labels: x, then y.
    tick_labels: Vec<PlacedText>,
    x_offset_label: Option<PlacedText>,
    y_offset_label: Option<PlacedText>,
}

impl Layout {
    /// Every piece of text in the layout.
    fn into_texts(self) -> impl Iterator<Item = PlacedText> {
        (self.tick_labels.into_iter())
            .chain(self.x_offset_label)
            .chain(self.y_offset_label)
    }
}

/// Text rasterized and positioned on the canvas.
#[derive(Debug, Clone)]
struct PlacedText {
    coverage: Coverage,
    color: RGB8,
    /// Column of the leftmost pixel.
    left: u32,
    /// Row of the top pixel (rows count up from the bottom of the canvas).
    top: u32,
}

impl PlacedText {
    /// `text` rasterized at `size` pixels with its top-left pixel at (`left`, `top`).
    fn new(font: &Font, text: &str, size: u32, color: RGB8, left: u32, top: u32) -> PlacedText {
        PlacedText {
            coverage: font.rasterize(text, size),
            color,
            left,
            top,
        }
    }

    /// A label placed by its [`TextPositioning`]: centered on the point or starting at it, and
    /// vertically centered on it either way.
    fn from_label(font: &Font, label: &Label, size: u32) -> PlacedText {
        let coverage = font.rasterize(label.text(), size);
        let point = label.pos().point();
        let left = match label.pos() {
            TextPositioning::Centered(_) => point.x.saturating_sub(coverage.width / 2),
            TextPositioning::LeftAligned(_) => point.x,
        };
        let top = point.y.saturating_add(coverage.height / 2);
        PlacedText {
            coverage,
            color: label.style().color(),
            left,
            top,
        }
    }

    /// The pixels it covers (inclusive), or `None` when it has none.
    #[cfg(test)]
    fn bounds(&self) -> Option<Limits<u32>> {
        if self.coverage.width == 0 || self.coverage.height == 0 {
            return None;
        }
        Some(Limits::new(
            Point::new(self.left, self.top.saturating_sub(self.coverage.height - 1)),
            Point::new(self.left.saturating_add(self.coverage.width - 1), self.top),
        ))
    }

    fn draw(&self, canvas: &mut Canvas) {
        for row in 0..self.coverage.height {
            let Some(y) = self.top.checked_sub(row) else {
                break;
            };
            for col in 0..self.coverage.width {
                let coverage = self.coverage.get(col, row);
                if coverage > 0 {
                    canvas.blend(self.left.saturating_add(col), y, self.color, coverage);
                }
            }
        }
    }
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
    font: Font,
    font_size: u32,
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
            font: Font::default(),
            font_size: DEFAULT_FONT_SIZE,
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

    /// Sets the font for all text. Characters it lacks are drawn with the built-in Go font.
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Sets the base text size in pixels (the em size), clamped to `1..=`[`MAX_FONT_SIZE`].
    /// Tick labels and any text without a size of its own use it. The default is
    /// [`DEFAULT_FONT_SIZE`].
    pub fn with_font_size(mut self, px: u32) -> Self {
        self.font_size = px.clamp(1, MAX_FONT_SIZE);
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

        let mut texts = Vec::new();
        if let Some(graph) = self.graph.take() {
            let view = graph.view_limits()?;
            let layout = self.layout(&graph, &view)?;
            let plot = layout.plot.convert_to_f64();
            let scaled_graph = graph.scale_with_view(&view, plot.clone())?;

            let mut masks = Vec::new();

            // grid lines first so the axes and data are drawn over them
            if let Some(grid_lines) = graph.grid_lines() {
                // tick values are relative to the axis offsets
                let (x_min, x_max) = (
                    view.min().x - layout.x_offset,
                    view.max().x - layout.x_offset,
                );
                let (y_min, y_max) = (
                    view.min().y - layout.y_offset,
                    view.max().y - layout.y_offset,
                );
                let xs: Vec<f64> = (layout.x_ticks.values.iter())
                    .map(|&v| to_canvas(v, x_min, x_max, plot.min().x, plot.max().x))
                    .collect();
                let ys: Vec<f64> = (layout.y_ticks.values.iter())
                    .map(|&v| to_canvas(v, y_min, y_max, plot.min().y, plot.max().y))
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
            texts.extend(layout.into_texts());
        }

        // user labels, then all text is drawn last so the graph doesn't cover it
        for label in &self.labels {
            let size = label.style().size().unwrap_or(self.font_size);
            texts.push(PlacedText::from_label(&self.font, label, size));
        }
        for text in &texts {
            text.draw(&mut self.canvas);
        }

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

    /// Color and size of tick labels: the axes' text style, at the canvas's base size when it
    /// has no size of its own. A label color identical to the background (e.g. the black default
    /// text on the default black canvas) is replaced with black or white, whichever contrasts
    /// with the background.
    fn label_style(&self, axes: Option<&Axes>) -> (RGB8, u32) {
        let style = axes.map(|a| *a.style()).unwrap_or_default();
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
        (color, style.size().unwrap_or(self.font_size))
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
                Some(AxesPositioning::XOnly(line)) => {
                    ((0, line.thickness().saturating_mul(2)), true, false)
                }
                Some(AxesPositioning::YOnly(line)) => {
                    ((line.thickness().saturating_mul(2), 0), false, true)
                }
                Some(AxesPositioning::XY(line)) => {
                    let inset = line.thickness().saturating_mul(2);
                    ((inset, inset), true, true)
                }
                None => ((0, 0), false, false),
            };
        // axes and markers can overlap, so use the larger of the two
        let inset_x = largest_marker_sz.max(axes_inset.0);
        let inset_y = largest_marker_sz.max(axes_inset.1);

        let (color, size) = self.label_style(axes.as_ref());
        let font = &self.font;
        let width = |text: &str| font.width(text, size).ceil() as u32;
        let place =
            |text: &str, left: u32, top: u32| PlacedText::new(font, text, size, color, left, top);
        let text_h = font.metrics(size).height();

        // Far from zero, f64 can't hold evenly spaced ticks, and the labels get long. As in
        // matplotlib, such an axis gets an offset (`+1e15`) that its ticks are fitted, placed and
        // labeled relative to. It's decided first because its label changes the margins.
        let x_offset = axis_offset(view.min().x, view.max().x);
        let y_offset = axis_offset(view.min().y, view.max().y);
        let (x_min, x_max) = (view.min().x - x_offset, view.max().x - x_offset);
        let (y_min, y_max) = (view.min().y - y_offset, view.max().y - y_offset);
        let x_offset_label = format_offset(x_offset).filter(|_| show_x_labels);
        let y_offset_label = format_offset(y_offset).filter(|_| show_y_labels);

        // x labels sit in a band along the bottom, with the x offset label on a line below them;
        // the top y label needs half a line above the plot, the y offset label a full line
        let x_band = if show_x_labels {
            text_h + x_offset_label.as_ref().map_or(0, |_| text_h + 1)
        } else {
            0
        };
        let bottom = if show_x_labels { x_band + LABEL_GAP } else { 0 };
        let top = if y_offset_label.is_some() {
            text_h + LABEL_GAP
        } else if show_y_labels {
            text_h / 2
        } else {
            0
        };
        // saturating: huge markers, lines or buffers must end in CanvasTooSmall, not overflow
        let plot_min_y = outer_min.y.saturating_add(bottom).saturating_add(inset_y);
        let plot_max_y = outer_max.y.saturating_sub(top.saturating_add(inset_y));
        check_area(
            &Point::new(outer_min.x, plot_min_y),
            &Point::new(outer_max.x, plot_max_y),
        )?;

        let (canvas_max_x, canvas_max_y) = (self.limits.max().x, self.limits.max().y);
        // y labels stay above the x label band and inside the canvas
        let y_lo = if show_x_labels {
            outer_min.y + x_band + 1 + text_h / 2
        } else {
            text_h / 2
        };
        let y_hi = canvas_max_y.saturating_sub(text_h - text_h / 2).max(y_lo);
        let y_label_center = |value: f64| {
            let y = to_canvas(value, y_min, y_max, plot_min_y as f64, plot_max_y as f64);
            (y.round() as u32).clamp(y_lo, y_hi)
        };

        // y ticks depend only on the plot height; their labels then set the left margin
        let y_ticks = fit_ticks(
            y_min,
            y_max,
            (plot_max_y - plot_min_y) as f64,
            |ticks, spacing| {
                // the clamp above can push end labels into their neighbours
                let centers: Vec<u32> = ticks.values.iter().map(|&v| y_label_center(v)).collect();
                spacing >= text_h as f64 + Y_LABEL_SPACING
                    && centers.windows(2).all(|c| c[1] >= c[0] + text_h)
            },
        );
        let y_label_w = if show_y_labels {
            (y_ticks.labels.iter()).map(|l| width(l)).max().unwrap_or(0)
        } else {
            0
        };
        let left = if show_y_labels {
            y_label_w + LABEL_GAP
        } else {
            0
        };
        let plot_min_x = outer_min.x.saturating_add(left).saturating_add(inset_x);
        let plot_max_x = outer_max.x.saturating_sub(inset_x);
        let plot_min = Point::new(plot_min_x, plot_min_y);
        let plot_max = Point::new(plot_max_x, plot_max_y);
        check_area(&plot_min, &plot_max)?;

        let x_label_center = |value: f64, w: u32| {
            let x = to_canvas(value, x_min, x_max, plot_min_x as f64, plot_max_x as f64);
            // keep labels at the ends of the axis inside the canvas
            let lo = w / 2;
            let hi = canvas_max_x.saturating_sub(w - w / 2).max(lo);
            (x.round() as u32).clamp(lo, hi)
        };
        let x_ticks = fit_ticks(
            x_min,
            x_max,
            (plot_max_x - plot_min_x) as f64,
            |ticks, spacing| {
                let widths: Vec<u32> = ticks.labels.iter().map(|l| width(l)).collect();
                let widest = widths.iter().copied().max().unwrap_or(0);
                // the clamp in x_label_center can push end labels into their neighbours
                let spans: Vec<(u32, u32)> = (ticks.values.iter().zip(&widths))
                    .map(|(&v, &w)| {
                        let left = x_label_center(v, w) - w / 2;
                        (left, left + w)
                    })
                    .collect();
                spacing >= widest as f64 + X_LABEL_SPACING
                    && spans
                        .windows(2)
                        .all(|s| s[1].0 as f64 >= s[0].1 as f64 + X_LABEL_SPACING / 2.0)
            },
        );

        let plot = Limits::new(plot_min, plot_max);
        let mut tick_labels = Vec::new();
        if show_x_labels {
            // the top line of the band
            let top = outer_min.y + x_band - 1;
            for (&value, label) in x_ticks.values.iter().zip(&x_ticks.labels) {
                let w = width(label);
                tick_labels.push(place(label, x_label_center(value, w) - w / 2, top));
            }
        }
        if show_y_labels {
            for (&value, label) in y_ticks.values.iter().zip(&y_ticks.labels) {
                // right-aligned against the plot area, vertically centered on the tick
                let left = outer_min.x + y_label_w - width(label);
                tick_labels.push(place(label, left, y_label_center(value) + text_h / 2));
            }
        }

        // where matplotlib puts them: the x offset under the right end of the x axis, the y
        // offset above the top of the y axis
        let x_offset_label = x_offset_label.map(|text| {
            let left = (plot_max_x + 1).saturating_sub(width(&text));
            place(&text, left, outer_min.y + text_h - 1)
        });
        let y_offset_label = y_offset_label
            .map(|text| place(&text, plot_min_x.saturating_sub(axes_inset.0), outer_max.y));

        Ok(Layout {
            plot,
            x_ticks,
            y_ticks,
            x_offset,
            y_offset,
            tick_labels,
            x_offset_label,
            y_offset_label,
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
    use crate::plotting::{colors, series::Series, text::TextStyle};

    #[test]
    fn blend_mixes_into_one_pixel() {
        let mut canvas = Canvas::new(2, 2, colors::BLACK);
        canvas.blend(0, 0, colors::WHITE, 128);
        canvas.blend(5, 5, colors::WHITE, 255); // outside: ignored
        // (0, 0) is the lower-left pixel, stored in the last row
        assert_eq!(
            canvas.get_bytes(),
            [0, 0, 0, 0, 0, 0, 188, 188, 188, 0, 0, 0]
        );
    }

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

    fn white_axes() -> Axes {
        use crate::plotting::line::LineStyle;
        Axes::new(
            AxesPositioning::XY(LineStyle::solid(colors::WHITE, 1)),
            TextStyle::with_color(colors::WHITE),
        )
    }

    #[test]
    fn ticks_near_1e15_use_an_offset() {
        // f64 values are 0.125 apart here, so 1e15 + 0.1 * i can't be stored exactly
        let points: Vec<_> = (0..16)
            .map(|i| Point::new(i as f64, 1e15 + 0.1 * i as f64))
            .collect();
        let graph = Graph::new()
            .with_series(Series::new(&points))
            .with_axes(white_axes());
        let canvas = TerminalCanvas::new(700, 700, colors::BLACK)
            .with_buffer(BufferType::Uniform(8))
            .with_graph(graph.clone());
        let layout = canvas
            .layout(&graph, &graph.view_limits().unwrap())
            .unwrap();
        assert_eq!(layout.y_offset, 1e15);
        assert_eq!(
            layout.y_ticks.labels,
            ["0", "0.2", "0.4", "0.6", "0.8", "1.0", "1.2", "1.4", "1.6"]
        );
        assert_eq!(layout.x_offset, 0.0);
    }

    #[test]
    fn labels_are_drawn_with_anti_aliased_text() {
        let label = Label::new(
            "0",
            TextStyle::new(colors::WHITE, 28),
            TextPositioning::Centered(Point::new(20, 20)),
        );
        let bytes = TerminalCanvas::new(40, 40, colors::BLACK)
            .with_label(label)
            .draw()
            .unwrap()
            .get_bytes();
        let reds: Vec<u8> = bytes.chunks(3).map(|px| px[0]).collect();
        assert!(reds.contains(&255), "fully covered pixels");
        assert!(
            reds.iter().any(|&v| v > 0 && v < 255),
            "partly covered edge pixels"
        );
    }

    #[test]
    fn the_base_font_size_sets_the_tick_label_size() {
        let graph = Graph::new()
            .with_series(Series::new(&[Point::new(0.0, 0.0), Point::new(10.0, 10.0)]))
            .with_axes(white_axes());
        let layout_at = |size| {
            TerminalCanvas::new(800, 600, colors::BLACK)
                .with_font_size(size)
                .with_graph(graph.clone())
                .layout(&graph, &graph.view_limits().unwrap())
                .unwrap()
        };
        let (small, large) = (layout_at(14), layout_at(28));
        assert_eq!(
            small.tick_labels[0].coverage.height,
            Font::default().metrics(14).height()
        );
        assert_eq!(
            large.tick_labels[0].coverage.height,
            Font::default().metrics(28).height()
        );
        // bigger labels leave less room for the plot
        assert!(large.plot.span().1 < small.plot.span().1);
    }

    #[test]
    fn offset_labels_stay_clear_of_other_labels() {
        // timestamps on x and values near 1e15 on y give both axes an offset label
        let points: Vec<_> = (0..=10)
            .map(|i| Point::new(1_700_000_000.0 + 10.0 * i as f64, 1e15 + 0.1 * i as f64))
            .collect();
        let graph = Graph::new()
            .with_series(Series::new(&points))
            .with_axes(white_axes());
        for (w, h) in [(800, 600), (500, 300), (300, 200)] {
            let canvas = TerminalCanvas::new(w, h, colors::BLACK)
                .with_buffer(BufferType::Uniform(8))
                .with_graph(graph.clone());
            let layout = canvas
                .layout(&graph, &graph.view_limits().unwrap())
                .unwrap();
            let offsets: Vec<_> = (layout.x_offset_label.iter())
                .chain(&layout.y_offset_label)
                .map(|t| t.bounds().unwrap())
                .collect();
            assert_eq!(offsets.len(), 2, "{w}x{h}: offset labels");
            let ticks: Vec<_> = layout
                .tick_labels
                .iter()
                .map(|t| t.bounds().unwrap())
                .collect();
            for offset in &offsets {
                assert!(
                    offset.max().x < w && offset.max().y < h,
                    "{w}x{h}: {offset:?}"
                );
                for other in ticks.iter().chain(offsets.iter().filter(|o| *o != offset)) {
                    assert!(
                        !offset.intersects(other.clone()),
                        "{w}x{h}: {offset:?} overlaps {other:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn tick_labels_never_overlap() {
        use crate::plotting::line::LineStyle;
        let axes = Axes::new(
            AxesPositioning::XY(LineStyle::solid(colors::WHITE, 1)),
            TextStyle::with_color(colors::WHITE),
        );
        // timestamps: long labels, and the last one is pushed inward at the right edge
        for (w, h) in [(800, 600), (500, 300), (300, 200)] {
            let points = [
                (1_700_000_000, 1.0),
                (1_700_000_050, 2.0),
                (1_700_000_100, 3.0),
            ];
            let graph = Graph::new()
                .with_series(Series::new(&points.map(|(x, y)| Point::new(x as f64, y))))
                .with_axes(axes.clone());
            let canvas = TerminalCanvas::new(w, h, colors::BLACK)
                .with_buffer(BufferType::Uniform(8))
                .with_graph(graph.clone());
            let layout = canvas
                .layout(&graph, &graph.view_limits().unwrap())
                .unwrap();
            let x_labels: Vec<_> = (layout.tick_labels.iter())
                .take(layout.x_ticks.labels.len())
                .map(|l| l.bounds().unwrap())
                .collect();
            assert!(x_labels.len() >= 2, "{w}x{h}");
            for pair in x_labels.windows(2) {
                assert!(
                    pair[1].min().x > pair[0].max().x,
                    "{w}x{h}: {:?} overlaps {:?}",
                    pair[0],
                    pair[1]
                );
            }
        }
    }
}
