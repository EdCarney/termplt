use super::{
    axes::{Axes, AxesPositioning},
    colors,
    common::FloatConvertable,
    font::{Coverage, Font},
    graph::Graph,
    limits::Limits,
    point::Point,
    srgb,
    text::{DEFAULT_FONT_SIZE, Label, MAX_FONT_SIZE, TextPositioning, wrap},
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

/// Most lines a title wraps onto.
const TITLE_MAX_LINES: usize = 3;
/// Most lines an axis name wraps onto.
const NAME_MAX_LINES: usize = 2;
/// Title size relative to the tick labels (matplotlib's `large`).
const TITLE_SCALE: f32 = 1.2;
/// Baseline-to-baseline distance of wrapped lines, in em (matplotlib's `linespacing`).
const LINE_SPACING: f32 = 1.2;
/// Gap between the tick labels and the plot, in em (matplotlib's `xtick.major.pad`).
const TICK_GAP: f32 = 0.35;
/// Gap between an axis name and the tick labels, in em (matplotlib's `axes.labelpad`).
const NAME_GAP: f32 = 0.4;
/// Gap between the title and the plot, in em (matplotlib's `axes.titlepad`).
const TITLE_GAP: f32 = 0.6;
/// Gap around an offset label, in em (matplotlib's `Axis.OFFSETTEXTPAD`).
const OFFSET_GAP: f32 = 0.3;
/// Text sharing a line and closer than this, in em, is moved onto separate lines.
const MIN_APART: f32 = 0.5;
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
    /// Title lines, first on top.
    title: Vec<PlacedText>,
    /// x axis name lines, first on top.
    x_name: Vec<PlacedText>,
    /// y axis name lines turned to read upwards, first on the left.
    y_name: Vec<PlacedText>,
}

impl Layout {
    /// Every piece of text in the layout.
    fn into_texts(self) -> impl Iterator<Item = PlacedText> {
        (self.tick_labels.into_iter())
            .chain(self.x_offset_label)
            .chain(self.y_offset_label)
            .chain(self.title)
            .chain(self.x_name)
            .chain(self.y_name)
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
        for (x, y, coverage) in self.coverage.ink() {
            // the bitmap's rows count down, the canvas's count up
            let (x, y) = (i64::from(self.left) + x, i64::from(self.top) - y);
            if let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) {
                canvas.blend(x, y, self.color, coverage);
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

    /// Lays out the graph. Text that normally shares a line (the title and the y offset, the x
    /// axis name and the x offset) gets a line of its own when the two would collide.
    fn layout(&self, graph: &Graph, view: &Limits<f64>) -> Result<Layout> {
        // Stacking one pair shrinks the plot, which re-wraps the y name and moves the other
        // pair, so it can cause a new collision. Stacked pairs stay stacked, so this settles
        // within three passes.
        let mut stacking = Stacking::default();
        loop {
            let (layout, collisions) = self.layout_with(graph, view, stacking)?;
            let next = Stacking {
                title: stacking.title || collisions.title,
                x_offset: stacking.x_offset || collisions.x_offset,
            };
            if next == stacking {
                return Ok(layout);
            }
            stacking = next;
        }
    }

    /// One layout pass with `stacking` applied; also returns the shared lines that collide.
    fn layout_with(
        &self,
        graph: &Graph,
        view: &Limits<f64>,
        stacking: Stacking,
    ) -> Result<(Layout, Stacking)> {
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
        let title_size = ((size as f32 * TITLE_SCALE).round() as u32).clamp(1, MAX_FONT_SIZE);
        let font = &self.font;
        let width = |text: &str, size: u32| font.width(text, size).ceil() as u32;
        let place = |text: &str, size: u32, left: u32, top: u32| {
            PlacedText::new(font, text, size, color, left, top)
        };
        let tick = font.metrics(size);
        let text_h = tick.height();
        let title_h = font.metrics(title_size).height();
        // gaps and line spacing scale with the text: em(0.35) is 0.35 of the tick label size
        let em = |fraction: f32| (fraction * size as f32).round() as u32;
        let (tick_gap, name_gap) = (em(TICK_GAP), em(NAME_GAP));
        let (title_gap, offset_gap) = (em(TITLE_GAP), em(OFFSET_GAP));
        let pitch = |size: u32| (size as f32 * LINE_SPACING).round() as u32;
        // height of a block of `lines` wrapped lines, each `line_h` tall
        let block = |lines: usize, line_h: u32, size: u32| match lines {
            0 => 0,
            n => (n as u32 - 1) * pitch(size) + line_h,
        };

        // Far from zero, f64 can't hold evenly spaced ticks, and the labels get long. As in
        // matplotlib, such an axis gets an offset (`+1e15`) that its ticks are fitted, placed and
        // labeled relative to. It's decided first because its label changes the margins.
        let x_offset = axis_offset(view.min().x, view.max().x);
        let y_offset = axis_offset(view.min().y, view.max().y);
        let (x_min, x_max) = (view.min().x - x_offset, view.max().x - x_offset);
        let (y_min, y_max) = (view.min().y - y_offset, view.max().y - y_offset);
        let x_offset_text = format_offset(x_offset).filter(|_| show_x_labels);
        let y_offset_text = format_offset(y_offset).filter(|_| show_y_labels);

        // the title and x name wrap to the width inside the buffer; as with matplotlib's
        // `axis("off")`, an axis without tick labels has no name either
        let outer_w = (outer_max.x.saturating_sub(outer_min.x) + 1) as f32;
        let title_lines = graph.title().map_or_else(Vec::new, |t| {
            wrap(t, outer_w, TITLE_MAX_LINES, |s| font.width(s, title_size))
        });
        let x_name_lines = match graph.x_label() {
            Some(t) if show_x_labels => wrap(t, outer_w, NAME_MAX_LINES, |s| font.width(s, size)),
            _ => Vec::new(),
        };
        let title_block = block(title_lines.len(), title_h, title_size);
        let x_name_block = block(x_name_lines.len(), text_h, size);

        // bottom band, from the edge up: the x offset when it has a line of its own, the x
        // name, then the tick labels
        let x_offset_alone =
            x_offset_text.is_some() && (x_name_lines.is_empty() || stacking.x_offset);
        let x_offset_line = if x_offset_alone {
            text_h + offset_gap
        } else {
            0
        };
        let x_name_band = if x_name_lines.is_empty() {
            0
        } else {
            x_name_block + name_gap
        };
        let below_ticks = x_offset_line + x_name_band;
        let bottom = if show_x_labels {
            below_ticks + text_h + tick_gap
        } else {
            0
        };

        // top band, from the plot up: the y offset, and the title on the same line unless the
        // two are stacked; the title also clears the top y tick label, which reaches half a
        // line above the plot
        let title_lift = if y_offset_text.is_some() && stacking.title {
            offset_gap + text_h + title_gap
        } else {
            title_gap.max(text_h / 2 + 1)
        };
        let y_offset_band = if y_offset_text.is_some() {
            offset_gap + text_h
        } else {
            0
        };
        let top = if !title_lines.is_empty() {
            (title_lift + title_block).max(y_offset_band)
        } else if y_offset_text.is_some() {
            y_offset_band
        } else if show_y_labels {
            text_h / 2
        } else {
            0
        };

        // saturating: huge markers, lines, text or buffers must end in CanvasTooSmall, not
        // overflow
        let plot_min_y = outer_min.y.saturating_add(bottom).saturating_add(inset_y);
        let plot_max_y = outer_max.y.saturating_sub(top.saturating_add(inset_y));
        check_area(
            &Point::new(outer_min.x, plot_min_y),
            &Point::new(outer_max.x, plot_max_y),
        )?;

        let (canvas_max_x, canvas_max_y) = (self.limits.max().x, self.limits.max().y);
        // the top row of the x tick labels
        let x_labels_top = outer_min.y + below_ticks + text_h - 1;
        // y tick labels are centered on their tick by the digits (matplotlib's
        // `center_baseline`), stay inside the canvas, and stay a row above the x tick labels
        let digit_center = (tick.ascent - tick.digit_height / 2.0).round() as u32;
        let top_lo = if show_x_labels {
            x_labels_top + text_h + 1
        } else {
            text_h.saturating_sub(1)
        };
        let top_hi = canvas_max_y.max(top_lo);
        let y_label_top = |value: f64| {
            let y = to_canvas(value, y_min, y_max, plot_min_y as f64, plot_max_y as f64);
            (y.round() as u32)
                .saturating_add(digit_center)
                .clamp(top_lo, top_hi)
        };

        // y ticks depend only on the plot height; their labels then set the left margin
        let y_ticks = fit_ticks(
            y_min,
            y_max,
            (plot_max_y - plot_min_y) as f64,
            |ticks, spacing| {
                // the clamp above can push end labels into their neighbours
                let tops: Vec<u32> = ticks.values.iter().map(|&v| y_label_top(v)).collect();
                spacing >= text_h as f64 + Y_LABEL_SPACING
                    && tops.windows(2).all(|t| t[1] >= t[0] + text_h)
            },
        );

        // left band: the y name, wrapped to the plot height and turned to read upwards, then
        // the y tick labels
        let plot_h = (plot_max_y - plot_min_y + 1) as f32;
        let y_name_lines = match graph.y_label() {
            Some(t) if show_y_labels => wrap(t, plot_h, NAME_MAX_LINES, |s| font.width(s, size)),
            _ => Vec::new(),
        };
        let y_name_band = if y_name_lines.is_empty() {
            0
        } else {
            block(y_name_lines.len(), text_h, size) + name_gap
        };
        let y_label_w = if show_y_labels {
            (y_ticks.labels.iter())
                .map(|l| width(l, size))
                .max()
                .unwrap_or(0)
        } else {
            0
        };
        let left = if show_y_labels {
            y_name_band + y_label_w + tick_gap
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
                let widths: Vec<u32> = ticks.labels.iter().map(|l| width(l, size)).collect();
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
            for (&value, label) in x_ticks.values.iter().zip(&x_ticks.labels) {
                let w = width(label, size);
                tick_labels.push(place(
                    label,
                    size,
                    x_label_center(value, w) - w / 2,
                    x_labels_top,
                ));
            }
        }
        if show_y_labels {
            // right-aligned against the plot area
            let right = outer_min.x + y_name_band + y_label_w;
            for (&value, label) in y_ticks.values.iter().zip(&y_ticks.labels) {
                let left = right - width(label, size);
                tick_labels.push(place(label, size, left, y_label_top(value)));
            }
        }

        // the title and x name are centered on the plot; a block too wide for that shifts as a
        // whole to stay inside the buffered area, so its lines stay centered on each other
        let plot_center_x = (plot_min_x + plot_max_x) / 2;
        let block_center = |widths: &[u32]| {
            let widest = widths.iter().copied().max().unwrap_or(0);
            let left = (plot_center_x.saturating_sub(widest / 2))
                .min((outer_max.x + 1).saturating_sub(widest))
                .max(outer_min.x);
            left + widest / 2
        };

        // the x name below the tick labels, first line on top
        let x_name_top = x_labels_top.saturating_sub(text_h + name_gap);
        let x_name_widths: Vec<u32> = x_name_lines.iter().map(|l| width(l, size)).collect();
        let x_name_center = block_center(&x_name_widths);
        let x_name: Vec<PlacedText> = (x_name_lines.iter().zip(&x_name_widths).enumerate())
            .map(|(i, (line, &w))| {
                let top = x_name_top.saturating_sub(i as u32 * pitch(size));
                place(line, size, x_name_center.saturating_sub(w / 2), top)
            })
            .collect();
        // where matplotlib puts it: under the right end of the x axis, on the x name's first
        // line or on a line of its own
        let x_offset_label = x_offset_text.map(|text| {
            let top = if x_offset_alone {
                outer_min.y + text_h - 1
            } else {
                x_name_top
            };
            place(
                &text,
                size,
                (plot_max_x + 1).saturating_sub(width(&text, size)),
                top,
            )
        });

        // above the plot: the y offset at the y axis, and the title, first line on top
        let band_start = plot_max_y + inset_y + 1;
        let y_offset_label = y_offset_text.map(|text| {
            let left = plot_min_x.saturating_sub(axes_inset.0);
            place(&text, size, left, band_start + offset_gap + text_h - 1)
        });
        let title_top = band_start + title_lift + title_block.saturating_sub(1);
        let title_widths: Vec<u32> = title_lines.iter().map(|l| width(l, title_size)).collect();
        let title_center = block_center(&title_widths);
        let title: Vec<PlacedText> = (title_lines.iter().zip(&title_widths).enumerate())
            .map(|(i, (line, &w))| {
                let top = title_top.saturating_sub(i as u32 * pitch(title_size));
                place(line, title_size, title_center.saturating_sub(w / 2), top)
            })
            .collect();

        // the y name: each line turned to read upwards, the first farthest from the plot, all
        // centered on the plot vertically
        let plot_center_y = (plot_min_y + plot_max_y) / 2;
        let y_name: Vec<PlacedText> = (y_name_lines.iter().enumerate())
            .map(|(i, line)| {
                let mut text = place(line, size, outer_min.x + i as u32 * pitch(size), 0);
                text.coverage = text.coverage.rotated_ccw();
                text.top = plot_center_y + text.coverage.height / 2;
                text
            })
            .collect();

        // shared lines whose text would come closer than MIN_APART
        let apart = em(MIN_APART);
        let collide = |a: Option<&PlacedText>, b: Option<&PlacedText>| match (
            a.and_then(PlacedText::bounds),
            b.and_then(PlacedText::bounds),
        ) {
            (Some(a), Some(b)) => horizontal_gap(&a, &b) < apart,
            _ => false,
        };
        let collisions = Stacking {
            title: !stacking.title && collide(title.last(), y_offset_label.as_ref()),
            x_offset: !x_offset_alone && collide(x_name.first(), x_offset_label.as_ref()),
        };

        Ok((
            Layout {
                plot,
                x_ticks,
                y_ticks,
                x_offset,
                y_offset,
                tick_labels,
                x_offset_label,
                y_offset_label,
                title,
                x_name,
                y_name,
            },
            collisions,
        ))
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

/// Which text that normally shares a line gets a line of its own: the title (above the y
/// offset) and the x offset (below the x axis name).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Stacking {
    title: bool,
    x_offset: bool,
}

/// Columns of empty space between two boxes side by side; 0 when their columns overlap.
fn horizontal_gap(a: &Limits<u32>, b: &Limits<u32>) -> u32 {
    if a.max().x < b.min().x {
        b.min().x - a.max().x - 1
    } else if b.max().x < a.min().x {
        a.min().x - b.max().x - 1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plotting::{colors, marker::MarkerStyle, series::Series, text::TextStyle};

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
    fn ink_left_of_the_pen_is_drawn() {
        // Go's j reaches about 1.6 px left of its pen position at 40 px
        let label = Label::new(
            "j",
            TextStyle::new(colors::WHITE, 40),
            TextPositioning::LeftAligned(Point::new(20, 30)),
        );
        let bytes = TerminalCanvas::new(60, 60, colors::BLACK)
            .with_label(label)
            .draw()
            .unwrap()
            .get_bytes();
        let lit_left = (bytes.chunks(3).enumerate())
            .filter(|(i, px)| i % 60 < 20 && px[0] > 0)
            .count();
        assert!(lit_left > 0, "the tail of the j is cut off");
    }

    #[test]
    fn ink_above_the_ascent_is_drawn() {
        // this font's line box is shorter than its digits, as many fonts' boxes are shorter
        // than their accented capitals (Å, É)
        let font =
            Font::from_bytes(include_bytes!("../../tests/fixtures/short-ascent.ttf").to_vec())
                .unwrap();
        let size = 40;
        // the box's top row, as a label centered on row 40 is placed
        let top = 40 + font.metrics(size).height() / 2;
        let label = Label::new(
            "0",
            TextStyle::new(colors::WHITE, size),
            TextPositioning::LeftAligned(Point::new(20, 40)),
        );
        let bytes = TerminalCanvas::new(100, 100, colors::BLACK)
            .with_font(font)
            .with_label(label)
            .draw()
            .unwrap()
            .get_bytes();
        // bytes are stored from the top row; canvas rows count up from the bottom
        let lit_above = (bytes.chunks(3).enumerate())
            .filter(|(i, px)| 99 - (i / 100) as u32 > top && px[0] > 0)
            .count();
        assert!(lit_above > 0, "the top of the 0 is cut off above row {top}");
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

    /// A series that draws nothing itself, so the plot's inset is just the 2-row axis.
    fn plain_series() -> Series {
        Series::new(&[Point::new(0.0, 0.0), Point::new(10.0, 10.0)])
            .with_marker_style(MarkerStyle::None)
    }

    fn plain_graph() -> Graph {
        Graph::new()
            .with_series(plain_series())
            .with_axes(white_axes())
    }

    fn layout_of(graph: &Graph, (w, h): (u32, u32), font_size: u32) -> Layout {
        TerminalCanvas::new(w, h, colors::BLACK)
            .with_buffer(BufferType::Uniform(8))
            .with_font_size(font_size)
            .with_graph(graph.clone())
            .layout(graph, &graph.view_limits().unwrap())
            .unwrap()
    }

    /// Values near 1e15: the y axis gets an offset label above its top-left end.
    fn near_1e15() -> Vec<Point<f64>> {
        (0..=10)
            .map(|i| Point::new(i as f64, 1e15 + 0.1 * i as f64))
            .collect()
    }

    /// Timestamps: the x axis gets an offset label under its right end.
    fn timestamps() -> Vec<Point<f64>> {
        (0..=10)
            .map(|i| Point::new(1_700_000_000.0 + 10.0 * i as f64, i as f64))
            .collect()
    }

    /// Words that fill most of `width` pixels at `size`.
    fn line_filling(width: u32, size: u32) -> String {
        let font = Font::default();
        let mut text = String::from("Wide");
        while font.width(&format!("{text} title"), size) < width as f32 - 30.0 {
            text.push_str(" title");
        }
        text
    }

    #[test]
    fn tick_labels_sit_a_third_of_an_em_below_the_plot() {
        let layout = layout_of(&plain_graph(), (800, 600), 20);
        // 0.35 em at 20 px is 7 rows, below the 2-row axis inset
        assert_eq!(layout.tick_labels[0].top, layout.plot.min().y - 2 - 7 - 1);
    }

    #[test]
    fn y_tick_labels_are_centered_on_their_tick_by_the_digits() {
        let graph = plain_graph();
        let view = graph.view_limits().unwrap();
        let layout = layout_of(&graph, (800, 600), 28);
        let x_count = layout.x_ticks.labels.len();
        let (plot_min, plot_max) = (layout.plot.min().y as f64, layout.plot.max().y as f64);
        for (label, &value) in layout.tick_labels[x_count..]
            .iter()
            .zip(&layout.y_ticks.values)
        {
            let c = &label.coverage;
            let inked: Vec<u32> = (0..c.height)
                .filter(|&row| (0..c.width).any(|col| c.get(col, row) > 0))
                .collect();
            let ink_middle = label.top as f64 - f64::from(inked[0] + inked[inked.len() - 1]) / 2.0;
            let tick = to_canvas(value, view.min().y, view.max().y, plot_min, plot_max);
            assert!(
                (ink_middle - tick).abs() <= 1.0,
                "label for {value}: ink centered at {ink_middle}, tick at {tick}"
            );
        }
    }

    #[test]
    fn a_title_and_axis_names_take_room_from_the_plot() {
        let base = layout_of(&plain_graph(), (800, 600), 14).plot;
        let titled = layout_of(&plain_graph().with_title("Title"), (800, 600), 14);
        assert_eq!(titled.title.len(), 1);
        assert!(titled.plot.max().y < base.max().y);
        let x_named = layout_of(&plain_graph().with_x_label("Time (s)"), (800, 600), 14);
        assert_eq!(x_named.x_name.len(), 1);
        assert!(x_named.plot.min().y > base.min().y);
        let y_named = layout_of(&plain_graph().with_y_label("Amplitude"), (800, 600), 14);
        assert_eq!(y_named.y_name.len(), 1);
        assert!(y_named.plot.min().x > base.min().x);
    }

    #[test]
    fn blank_text_takes_no_room() {
        let blank = plain_graph()
            .with_title("  ")
            .with_x_label("")
            .with_y_label(" \n ");
        let layout = layout_of(&blank, (800, 600), 14);
        assert!(layout.title.is_empty() && layout.x_name.is_empty() && layout.y_name.is_empty());
        assert_eq!(layout.plot, layout_of(&plain_graph(), (800, 600), 14).plot);
    }

    #[test]
    fn a_long_title_wraps_to_three_lines_inside_the_buffer() {
        let words = "the quick brown fox jumps over the lazy dog ".repeat(12);
        let layout = layout_of(&plain_graph().with_title(words), (400, 300), 14);
        assert_eq!(layout.title.len(), 3);
        for line in &layout.title {
            let b = line.bounds().unwrap();
            // the buffered area of a 400-pixel canvas with an 8-pixel buffer is columns 8-391
            assert!(
                b.min().x >= 8 && b.max().x <= 391,
                "{b:?} leaves the buffered area"
            );
        }
    }

    #[test]
    fn wrapped_lines_share_one_center() {
        // the y name moves the plot's center right, so a full-width title has to shift left:
        // all its lines shift together and stay centered on each other
        let words = "the quick brown fox jumps over the lazy dog ".repeat(12);
        let graph = plain_graph()
            .with_title(words)
            .with_y_label("a y axis name");
        let layout = layout_of(&graph, (400, 300), 14);
        let centers: Vec<f64> = (layout.title.iter())
            .map(|line| line.left as f64 + line.coverage.width as f64 / 2.0)
            .collect();
        assert_eq!(centers.len(), 3);
        for c in &centers {
            assert!((c - centers[0]).abs() <= 1.0, "line centers {centers:?}");
        }
    }

    #[test]
    fn the_y_name_reads_upwards_beside_the_tick_labels() {
        let layout = layout_of(
            &plain_graph().with_y_label("Amplitude (µV)"),
            (800, 600),
            14,
        );
        let name = layout.y_name[0].bounds().unwrap();
        let (w, h) = name.span();
        assert!(h > w, "turned upright: {name:?}");
        let name_middle = (name.min().y + name.max().y) / 2;
        let plot_middle = (layout.plot.min().y + layout.plot.max().y) / 2;
        assert!(name_middle.abs_diff(plot_middle) <= 1);
        let first_y_label = layout.tick_labels[layout.x_ticks.labels.len()]
            .bounds()
            .unwrap();
        assert!(name.max().x < first_y_label.min().x);
    }

    #[test]
    fn the_title_shares_the_y_offset_line_until_they_would_meet() {
        let graph = Graph::new()
            .with_series(Series::new(&near_1e15()))
            .with_axes(white_axes());
        let short = layout_of(&graph.clone().with_title("T"), (800, 600), 14);
        let offset = short.y_offset_label.as_ref().unwrap().bounds().unwrap();
        let title = short.title[0].bounds().unwrap();
        assert!(
            title.min().y <= offset.max().y,
            "one band: {title:?} {offset:?}"
        );

        // a title almost as wide as the canvas reaches the offset at the left and moves up
        let wide = layout_of(&graph.with_title(line_filling(784, 17)), (800, 600), 14);
        assert_eq!(wide.title.len(), 1);
        let offset = wide.y_offset_label.as_ref().unwrap().bounds().unwrap();
        let title = wide.title[0].bounds().unwrap();
        assert!(
            title.min().y > offset.max().y,
            "stacked: {title:?} above {offset:?}"
        );
    }

    #[test]
    fn the_x_offset_shares_the_x_name_line_until_they_would_meet() {
        let graph = Graph::new()
            .with_series(Series::new(&timestamps()))
            .with_axes(white_axes());
        let short = layout_of(&graph.clone().with_x_label("t"), (800, 600), 14);
        assert_eq!(
            short.x_offset_label.as_ref().unwrap().top,
            short.x_name[0].top
        );

        let wide = layout_of(&graph.with_x_label(line_filling(784, 14)), (800, 600), 14);
        let offset = wide.x_offset_label.as_ref().unwrap().bounds().unwrap();
        let name = wide.x_name.last().unwrap().bounds().unwrap();
        assert!(
            offset.max().y < name.min().y,
            "stacked: {offset:?} below {name:?}"
        );
    }

    /// Fails when a shared line holds two texts closer than MIN_APART.
    fn assert_shared_lines_apart(layout: &Layout, font_size: u32) {
        let apart = (MIN_APART * font_size as f32).round() as u32;
        let pairs = [
            (layout.title.last(), layout.y_offset_label.as_ref()),
            (layout.x_name.first(), layout.x_offset_label.as_ref()),
        ];
        for (text, offset) in pairs {
            let (Some(text), Some(offset)) = (text, offset) else {
                continue;
            };
            let (a, b) = (text.bounds().unwrap(), offset.bounds().unwrap());
            let share_rows = a.min().y <= b.max().y && b.min().y <= a.max().y;
            if share_rows {
                let gap = horizontal_gap(&a, &b);
                assert!(gap >= apart, "{a:?} and {b:?} share a line {gap} px apart");
            }
        }
    }

    #[test]
    fn a_collision_that_appears_in_the_second_pass_is_stacked_too() {
        // stacking the title shrinks the plot, which re-wraps the y name and moves the x name
        // into the x offset
        let points: Vec<_> = (0..=10)
            .map(|i| Point::new(1_700_000_000.0 + 10.0 * i as f64, 1e15 + 0.1 * i as f64))
            .collect();
        let graph = Graph::new()
            .with_series(Series::new(&points))
            .with_axes(white_axes())
            .with_title("n".repeat(34))
            .with_x_label("m".repeat(25))
            .with_y_label("yyyy ".repeat(8));
        assert_shared_lines_apart(&layout_of(&graph, (400, 300), 12), 12);
    }

    proptest::proptest! {
        #[test]
        fn shared_lines_always_keep_their_distance(
            title in "[a-z ]{0,60}",
            x_label in "[a-z ]{0,60}",
            y_label in "[a-z ]{0,60}",
            w in 250u32..900,
            h in 200u32..700,
            font_size in 8u32..30,
        ) {
            // offsets on both axes, so both shared lines are in play
            let points: Vec<_> = (0..=10)
                .map(|i| Point::new(1_700_000_000.0 + 10.0 * i as f64, 1e15 + 0.1 * i as f64))
                .collect();
            let graph = Graph::new()
                .with_series(Series::new(&points))
                .with_axes(white_axes())
                .with_title(title)
                .with_x_label(x_label)
                .with_y_label(y_label);
            let canvas = TerminalCanvas::new(w, h, colors::BLACK)
                .with_buffer(BufferType::Uniform(8))
                .with_font_size(font_size)
                .with_graph(graph.clone());
            if let Ok(layout) = canvas.layout(&graph, &graph.view_limits().unwrap()) {
                assert_shared_lines_apart(&layout, font_size);
            }
        }
    }

    #[test]
    fn no_text_overlaps_other_text() {
        let points: Vec<_> = (0..=10)
            .map(|i| Point::new(1_700_000_000.0 + 10.0 * i as f64, 1e15 + 0.1 * i as f64))
            .collect();
        let graph = Graph::new()
            .with_series(Series::new(&points))
            .with_axes(white_axes())
            .with_title("Sensor drift")
            .with_x_label("Time (s)")
            .with_y_label("Reading (µV)");
        for (w, h) in [(800, 600), (500, 300), (300, 200)] {
            let layout = layout_of(&graph, (w, h), 14);
            assert!(layout.x_offset_label.is_some() && layout.y_offset_label.is_some());
            let boxes: Vec<Limits<u32>> = layout.into_texts().filter_map(|t| t.bounds()).collect();
            for (i, a) in boxes.iter().enumerate() {
                assert!(
                    a.max().x < w && a.max().y < h,
                    "{w}x{h}: {a:?} leaves the canvas"
                );
                for b in &boxes[i + 1..] {
                    assert!(!a.intersects(b.clone()), "{w}x{h}: {a:?} overlaps {b:?}");
                }
            }
        }
    }

    #[test]
    fn an_axis_without_tick_labels_has_no_name() {
        use crate::plotting::line::LineStyle;
        let y_only = Axes::new(
            AxesPositioning::YOnly(LineStyle::solid(colors::WHITE, 1)),
            TextStyle::with_color(colors::WHITE),
        );
        let graph = Graph::new()
            .with_series(plain_series())
            .with_axes(y_only)
            .with_x_label("x")
            .with_y_label("y");
        let layout = layout_of(&graph, (800, 600), 14);
        assert!(layout.x_name.is_empty());
        assert_eq!(layout.y_name.len(), 1);
    }

    proptest::proptest! {
        #[test]
        fn titles_and_names_stay_inside_the_canvas(
            title in "\\PC{0,80}",
            x_label in "\\PC{0,80}",
            y_label in "\\PC{0,80}",
            w in 150u32..900,
            h in 120u32..700,
            font_size in 6u32..40,
        ) {
            let graph = plain_graph()
                .with_title(title)
                .with_x_label(x_label)
                .with_y_label(y_label);
            let canvas = TerminalCanvas::new(w, h, colors::BLACK)
                .with_buffer(BufferType::Uniform(8))
                .with_font_size(font_size)
                .with_graph(graph.clone());
            // a canvas too small for the text is an error, which is fine
            if let Ok(layout) = canvas.layout(&graph, &graph.view_limits().unwrap()) {
                for text in layout.title.iter().chain(&layout.x_name).chain(&layout.y_name) {
                    if let Some(b) = text.bounds() {
                        proptest::prop_assert!(b.max().x < w && b.max().y < h, "{b:?} outside {w}x{h}");
                    }
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
