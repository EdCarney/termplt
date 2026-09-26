//! A one-stop builder for the common case: some series, axes, a grid, shown in the terminal or
//! saved as a PNG.

use crate::{
    Result,
    plotting::{
        axes::{Axes, AxesPositioning},
        canvas::{BufferType, TerminalCanvas},
        colors,
        font::Font,
        graph::Graph,
        grid_lines::GridLines,
        legend::LegendLocation,
        line::LineStyle,
        marker::MarkerStyle,
        series::Series,
        text::{DEFAULT_FONT_SIZE, TextStyle},
    },
    terminal::Terminal,
};
use rgb::RGB8;
use std::path::Path;

/// Size used by [`Plot::save_png`] when none is set.
pub const DEFAULT_PNG_SIZE: (u32, u32) = (800, 600);

/// A plot with sensible defaults: axes with tick labels, grid lines, a black background and a
/// color per series from [`colors::PALETTE`].
///
/// ```no_run
/// use termplt::Plot;
///
/// let xs: Vec<f64> = (0..100).map(|i| i as f64 / 10.0).collect();
/// let ys: Vec<f64> = xs.iter().map(|x| x.sin()).collect();
///
/// Plot::new()
///     .line((&xs, &ys))
///     .scatter(vec![(1.0, 0.5), (4.0, -0.5)])
///     .show()?; // sized to the terminal
/// # Ok::<(), termplt::Error>(())
/// ```
///
/// Anything that converts into a [`Series`] is accepted: `(xs, ys)` slices or vectors, a
/// `Vec` of `(x, y)` tuples, or a `Vec` of [`Point`](crate::plotting::point::Point)s, with any
/// primitive numeric type. For full control over styles, pass a styled [`Series`] to
/// [`Plot::series`], or use [`Plot::canvas`] to get the lower-level [`TerminalCanvas`].
#[derive(Debug, Clone, PartialEq)]
pub struct Plot {
    series: Vec<Series>,
    x_limits: Option<(f64, f64)>,
    y_limits: Option<(f64, f64)>,
    size: Option<(u32, u32)>,
    background: RGB8,
    grid: bool,
    title: Option<String>,
    x_label: Option<String>,
    y_label: Option<String>,
    font: Font,
    font_size: Option<u32>,
    legend: bool,
    legend_location: LegendLocation,
}

impl Default for Plot {
    fn default() -> Plot {
        Plot {
            series: Vec::new(),
            x_limits: None,
            y_limits: None,
            size: None,
            background: colors::BLACK,
            grid: true,
            title: None,
            x_label: None,
            y_label: None,
            font: Font::default(),
            font_size: None,
            legend: true,
            legend_location: LegendLocation::Best,
        }
    }
}

impl Plot {
    /// Creates an empty plot.
    pub fn new() -> Plot {
        Plot::default()
    }

    /// Adds a series drawn as a line without markers, in the next palette color.
    pub fn line(mut self, data: impl Into<Series>) -> Self {
        let color = self.next_color();
        let series = data
            .into()
            .with_marker_style(MarkerStyle::None)
            .with_line_style(LineStyle::solid(color, 0));
        self.series.push(series);
        self
    }

    /// Adds a series drawn as markers without a line, in the next palette color.
    pub fn scatter(mut self, data: impl Into<Series>) -> Self {
        let color = self.next_color();
        let series = data
            .into()
            .with_marker_style(MarkerStyle::FilledCircle { size: 2, color });
        self.series.push(series);
        self
    }

    /// Adds a series drawn as a line with a marker at each point, in the next palette color.
    pub fn line_points(mut self, data: impl Into<Series>) -> Self {
        let color = self.next_color();
        let series = data
            .into()
            .with_marker_style(MarkerStyle::FilledCircle { size: 2, color })
            .with_line_style(LineStyle::solid(color, 0));
        self.series.push(series);
        self
    }

    /// Adds a series with its own styles.
    pub fn series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    /// Fixes the x range; points outside it are not drawn, and lines break where they were.
    pub fn x_limits(mut self, min: f64, max: f64) -> Self {
        self.x_limits = Some((min, max));
        self
    }

    /// Fixes the y range; points outside it are not drawn, and lines break where they were.
    pub fn y_limits(mut self, min: f64, max: f64) -> Self {
        self.y_limits = Some((min, max));
        self
    }

    /// Sets the image size in pixels. By default [`Plot::show`] fits the terminal and
    /// [`Plot::save_png`] uses [`DEFAULT_PNG_SIZE`].
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.size = Some((width, height));
        self
    }

    /// Sets the background color. Axes, labels and the grid switch to dark colors on light
    /// backgrounds.
    pub fn background(mut self, color: RGB8) -> Self {
        self.background = color;
        self
    }

    /// Shows or hides the grid lines (shown by default).
    pub fn grid(mut self, show: bool) -> Self {
        self.grid = show;
        self
    }

    /// Sets the title, drawn above the plot. Long titles wrap onto up to 3 lines.
    pub fn title(mut self, text: impl Into<String>) -> Self {
        self.title = Some(text.into());
        self
    }

    /// Names the x axis.
    pub fn x_label(mut self, text: impl Into<String>) -> Self {
        self.x_label = Some(text.into());
        self
    }

    /// Names the y axis (drawn turned to read upwards).
    pub fn y_label(mut self, text: impl Into<String>) -> Self {
        self.y_label = Some(text.into());
        self
    }

    /// Sets the font for all text; characters it lacks are drawn with the built-in Go font.
    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Sets the base text size in pixels: tick labels and axis names use it, and the title is
    /// 1.2 times larger. By default [`Plot::show`] matches the terminal's text
    /// ([`Terminal::text_size`]), and other output uses [`DEFAULT_FONT_SIZE`].
    pub fn font_size(mut self, px: u32) -> Self {
        self.font_size = Some(px);
        self
    }

    /// Shows or hides the legend. It is shown by default whenever a series has a label: pass
    /// `Series::from(data).with_label("name")` to [`Plot::line`] and the others.
    pub fn legend(mut self, show: bool) -> Self {
        self.legend = show;
        self
    }

    /// Where the legend goes inside the plot. By default it's the location covering the least
    /// data, as matplotlib's `loc="best"` picks it.
    pub fn legend_location(mut self, location: LegendLocation) -> Self {
        self.legend_location = location;
        self
    }

    fn next_color(&self) -> RGB8 {
        colors::PALETTE[self.series.len() % colors::PALETTE.len()]
    }

    /// The graph with axes, grid and limits applied.
    pub fn graph(&self) -> Graph {
        let (foreground, grid_color) = if colors::luminance(self.background) > 127.5 {
            (colors::BLACK, colors::LIGHT_GRAY)
        } else {
            (colors::WHITE, colors::GRAY)
        };
        let mut graph = self
            .series
            .iter()
            .cloned()
            .fold(Graph::new(), Graph::with_series)
            .with_axes(Axes::new(
                AxesPositioning::XY(LineStyle::solid(foreground, 1)),
                TextStyle::with_color(foreground),
            ))
            .with_legend(self.legend)
            .with_legend_location(self.legend_location);
        if self.grid {
            graph = graph.with_grid_lines(GridLines::XY(LineStyle::solid(grid_color, 0)));
        }
        if let Some((min, max)) = self.x_limits {
            graph = graph.with_x_limits(min, max);
        }
        if let Some((min, max)) = self.y_limits {
            graph = graph.with_y_limits(min, max);
        }
        if let Some(text) = &self.title {
            graph = graph.with_title(text.clone());
        }
        if let Some(text) = &self.x_label {
            graph = graph.with_x_label(text.clone());
        }
        if let Some(text) = &self.y_label {
            graph = graph.with_y_label(text.clone());
        }
        graph
    }

    /// The canvas the plot is drawn on, for callers that want to adjust it before drawing.
    pub fn canvas(&self, width: u32, height: u32) -> TerminalCanvas {
        self.canvas_with_font_size(width, height, self.font_size.unwrap_or(DEFAULT_FONT_SIZE))
    }

    fn canvas_with_font_size(&self, width: u32, height: u32, font_size: u32) -> TerminalCanvas {
        // tick labels are laid out inside the canvas automatically; the buffer is just
        // breathing room around the edges
        let buffer = (width.min(height) / 40).max(8);
        TerminalCanvas::new(width, height, self.background)
            .with_buffer(BufferType::Uniform(buffer))
            .with_font(self.font.clone())
            .with_font_size(font_size)
            .with_graph(self.graph())
    }

    /// The base text size [`Plot::show_in`] uses: the one set, or the terminal's.
    fn font_size_in(&self, terminal: &Terminal) -> u32 {
        self.font_size.unwrap_or_else(|| terminal.text_size())
    }

    /// Draws the plot and returns its RGB8 pixels (`width * height * 3` bytes, row-major, top
    /// row first).
    pub fn render(&self, width: u32, height: u32) -> Result<Vec<u8>> {
        Ok(self.canvas(width, height).draw()?.into_bytes())
    }

    /// Draws the plot and saves it as a PNG file.
    pub fn save_png(&self, path: impl AsRef<Path>) -> Result<()> {
        let (width, height) = self.size.unwrap_or(DEFAULT_PNG_SIZE);
        let rgb = self.render(width, height)?;
        image::save_buffer_with_format(
            path,
            &rgb,
            width,
            height,
            image::ColorType::Rgb8,
            image::ImageFormat::Png,
        )?;
        Ok(())
    }

    /// Draws the plot in the terminal at the cursor, sized to fit the window unless a size was
    /// set. See [`Terminal::connect`] for the checks this runs first.
    pub fn show(&self) -> Result<()> {
        let terminal = Terminal::connect()?;
        self.show_in(&terminal)
    }

    /// Like [`Plot::show`], with a [`Terminal`] that was already connected.
    pub fn show_in(&self, terminal: &Terminal) -> Result<()> {
        let (width, height) = self.size.unwrap_or_else(|| terminal.default_plot_size());
        let rgb = self
            .canvas_with_font_size(width, height, self.font_size_in(terminal))
            .draw()?
            .into_bytes();
        terminal.show_rgb(&rgb, width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plotting::point::Point;

    #[test]
    fn legend_settings_reach_the_graph() {
        let plot = Plot::new().line(Series::from(vec![(0, 0), (1, 1)]).with_label("sin"));
        let graph = plot.graph();
        assert_eq!(graph.data()[0].label(), Some("sin"));
        assert!(graph.legend_visible());
        assert_eq!(graph.legend_location(), LegendLocation::Best);

        let graph = plot
            .legend(false)
            .legend_location(LegendLocation::UpperLeft)
            .graph();
        assert!(!graph.legend_visible());
        assert_eq!(graph.legend_location(), LegendLocation::UpperLeft);
    }

    #[test]
    fn titles_and_names_reach_the_graph() {
        let graph = Plot::new()
            .line(vec![(0, 0), (1, 1)])
            .title("T")
            .x_label("x")
            .y_label("y")
            .graph();
        assert_eq!(graph.title(), Some("T"));
        assert_eq!(graph.x_label(), Some("x"));
        assert_eq!(graph.y_label(), Some("y"));
    }

    #[test]
    fn font_size_sets_the_text_size_of_the_canvas() {
        let plot = Plot::new().line(vec![(0, 0), (1, 1)]);
        let area = |p: &Plot| p.canvas(800, 600).get_drawable_limits().unwrap().span();
        assert_eq!(
            area(&plot),
            area(&plot.clone().font_size(DEFAULT_FONT_SIZE))
        );
        // bigger text leaves less room for the plot
        assert!(area(&plot.clone().font_size(40)).1 < area(&plot).1);
    }

    #[test]
    fn show_matches_the_terminal_text_unless_a_size_is_set() {
        let terminal = Terminal::with_window(crate::terminal::WindowSize {
            rows: 50,
            cols: 160,
            x_pix: 1600,
            y_pix: 1700,
            pix_per_row: 34,
            pix_per_col: 10,
        });
        assert_eq!(Plot::new().font_size_in(&terminal), 28);
        assert_eq!(Plot::new().font_size(20).font_size_in(&terminal), 20);
    }

    #[test]
    fn series_get_palette_colors_in_order() {
        let plot = Plot::new()
            .line(vec![(0, 0), (1, 1)])
            .scatter(vec![(0, 1)])
            .line_points(vec![(1, 0)]);
        let graph = plot.graph();
        let series = graph.data();
        assert_eq!(series[0].line_style().unwrap().color(), colors::PALETTE[0]);
        assert_eq!(*series[0].marker_style(), MarkerStyle::None);
        assert!(series[1].line_style().is_none());
        assert_eq!(series[1].marker_style().color(), Some(colors::PALETTE[1]));
        assert_eq!(series[2].line_style().unwrap().color(), colors::PALETTE[2]);
    }

    #[test]
    fn input_shapes() {
        let xs = vec![0.0, 1.0, 2.0];
        let ys = vec![1u32, 3, 2];
        let plot = Plot::new()
            .line((&xs, &ys))
            .line((xs.as_slice(), ys.as_slice()))
            .line((xs.clone(), ys.clone()))
            .line(vec![Point::new(0, 1), Point::new(1, 3), Point::new(2, 2)])
            .line([(0, 1), (1, 3), (2, 2)])
            .line([(0, 1), (1, 3), (2, 2)])
            .line(vec![(0u8, 1i64), (1, 3), (2, 2)])
            .line(([0.0, 1.0, 2.0], [1, 3, 2]))
            .line((&[0.0, 1.0, 2.0], &[1, 3, 2]))
            .line(&[Point::new(0, 1), Point::new(1, 3), Point::new(2, 2)][..]);
        for series in plot.graph().data() {
            assert_eq!(series.data()[1], Point::new(1.0, 3.0));
        }
    }

    #[test]
    fn render_produces_rgb_of_the_right_size() {
        let rgb = Plot::new()
            .line(vec![(0, 0), (10, 5)])
            .render(200, 150)
            .unwrap();
        assert_eq!(rgb.len(), 200 * 150 * 3);
        // something other than the background was drawn
        assert!(rgb.iter().any(|&b| b != 0));
    }

    #[test]
    fn light_background_uses_dark_axes() {
        let graph = Plot::new()
            .background(colors::WHITE)
            .line(vec![(0, 0), (1, 1)])
            .graph();
        assert_eq!(graph.axes().unwrap().style().color(), colors::BLACK);
    }

    #[test]
    fn limits_and_grid_are_applied() {
        let graph = Plot::new()
            .line(vec![(0, 0), (1, 1)])
            .x_limits(-1.0, 2.0)
            .y_limits(0.0, 5.0)
            .grid(false)
            .graph();
        assert_eq!(graph.x_limits(), Some((-1.0, 2.0)));
        assert_eq!(graph.y_limits(), Some((0.0, 5.0)));
        assert!(graph.grid_lines().is_none());
    }

    #[test]
    fn empty_plot_is_an_error() {
        assert!(matches!(
            Plot::new().render(100, 100),
            Err(crate::Error::NoData)
        ));
    }

    #[test]
    fn save_png_writes_a_readable_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plot.png");
        Plot::new()
            .scatter(vec![(1, 2), (3, 4)])
            .size(320, 240)
            .save_png(&path)
            .unwrap();
        let img = image::open(&path).unwrap();
        assert_eq!((img.width(), img.height()), (320, 240));
    }
}
