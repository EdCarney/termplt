//! Several plots drawn as one image, on a grid.

use crate::{
    DEFAULT_PNG_SIZE, Error, Plot, Result,
    plot::Defaults,
    plotting::{colors, font::Font, text::DEFAULT_FONT_SIZE},
    terminal::Terminal,
};
use rgb::RGB8;
use std::{
    ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
    path::Path,
};

/// Several plots drawn as one image, on a grid of equal-size slots. A plot can cover one slot
/// or a block of them.
///
/// ```
/// use termplt::{Figure, Plot};
///
/// let points = vec![(0, 0), (1, 1), (2, 4)];
/// let fig = Figure::new(2, 2)
///     .plot(0, 0, Plot::new().line(points.clone()).title("line"))
///     .plot(0, 1, Plot::new().scatter(points.clone()).title("scatter"))
///     .plot(1, .., Plot::new().line_points(points).title("both"));
/// let rgb = fig.render(400, 300).unwrap();
/// assert_eq!(rgb.len(), 400 * 300 * 3);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Figure {
    rows: usize,
    cols: usize,
    plots: Vec<Plot>,
    /// Rows and columns each plot covers, resolved; same order as `plots`.
    spans: Vec<(Range<usize>, Range<usize>)>,
    size: Option<(u32, u32)>,
    background: RGB8,
    font: Font,
    font_size: Option<u32>,
}

impl Figure {
    /// A figure with `rows` x `cols` slots and no plots yet.
    pub fn new(rows: usize, cols: usize) -> Figure {
        Figure {
            rows,
            cols,
            plots: Vec::new(),
            spans: Vec::new(),
            size: None,
            background: colors::BLACK,
            font: Font::default(),
            font_size: None,
        }
    }

    /// Adds `plot` covering `rows` and `cols`: one index or a range of each, counted from the
    /// top-left slot. `..` means every row or column.
    pub fn plot(mut self, rows: impl GridSpan, cols: impl GridSpan, plot: Plot) -> Self {
        self.spans
            .push((rows.resolve(self.rows), cols.resolve(self.cols)));
        self.plots.push(plot);
        self
    }

    /// The image size in pixels. By default `show` fits the terminal and `save_png` uses
    /// `DEFAULT_PNG_SIZE`, as for `Plot`.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.size = Some((width, height));
        self
    }

    /// The background of slots without a plot, and of every plot without its own background.
    pub fn background(mut self, color: RGB8) -> Self {
        self.background = color;
        self
    }

    /// The font of every plot without its own font.
    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// The base text size of every plot without its own. By default `show` matches the
    /// terminal's text, and other output uses `DEFAULT_FONT_SIZE`, as for `Plot`.
    pub fn font_size(mut self, px: u32) -> Self {
        self.font_size = Some(px);
        self
    }

    /// The plots, in the order they were added.
    pub fn plots(&self) -> &[Plot] {
        &self.plots
    }

    /// The plots, in the order they were added, for changing their data in place (live figures).
    pub fn plots_mut(&mut self) -> &mut [Plot] {
        &mut self.plots
    }

    /// Draws the figure and returns its RGB8 pixels (`width * height * 3` bytes, row-major, top
    /// row first). The base text size is the figure's `font_size`, else [`DEFAULT_FONT_SIZE`].
    ///
    /// Fails if the grid is invalid, or if a plot's cell is too small to draw in.
    pub fn render(&self, width: u32, height: u32) -> Result<Vec<u8>> {
        self.render_at(width, height, self.font_size.unwrap_or(DEFAULT_FONT_SIZE))
    }

    /// Draws the figure and saves it as a PNG file, `size` pixels large or [`DEFAULT_PNG_SIZE`].
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

    /// Draws the figure in the terminal at the cursor, sized to fit the window unless a size
    /// was set. See [`Terminal::connect`] for the checks this runs first.
    pub fn show(&self) -> Result<()> {
        let terminal = Terminal::connect()?;
        self.show_in(&terminal)
    }

    /// Like [`Figure::show`], with a [`Terminal`] that was already connected.
    pub fn show_in(&self, terminal: &Terminal) -> Result<()> {
        let (width, height) = self.size_in(terminal);
        let rgb = self.render_at(width, height, self.font_size_in(terminal))?;
        terminal.show_rgb(&rgb, width, height)
    }

    /// Draws the figure with `font_size` as the base text size.
    pub(crate) fn render_at(&self, width: u32, height: u32, font_size: u32) -> Result<Vec<u8>> {
        self.validate()?;
        let background = [self.background.r, self.background.g, self.background.b];
        let stride = width as usize * 3;
        let mut rgb: Vec<u8> = background
            .iter()
            .copied()
            .cycle()
            .take(stride * height as usize)
            .collect();
        let defaults = Defaults {
            background: self.background,
            font: &self.font,
            font_size,
        };
        for (i, plot) in self.plots.iter().enumerate() {
            let (left, top, w, h) = self.cell_rect(i, width, height);
            let cell = plot.render_with(w, h, defaults)?;
            let row_bytes = w as usize * 3;
            for y in 0..h as usize {
                let start = (top as usize + y) * stride + left as usize * 3;
                rgb[start..start + row_bytes]
                    .copy_from_slice(&cell[y * row_bytes..(y + 1) * row_bytes]);
            }
        }
        Ok(rgb)
    }

    /// The image size: `size`, else the terminal's default plot size.
    pub(crate) fn size_in(&self, terminal: &Terminal) -> (u32, u32) {
        self.size.unwrap_or_else(|| terminal.default_plot_size())
    }

    /// The base text size: `font_size`, else the terminal's text size.
    pub(crate) fn font_size_in(&self, terminal: &Terminal) -> u32 {
        self.font_size.unwrap_or_else(|| terminal.text_size())
    }

    /// Checks the grid: it has rows and columns, every plot covers slots inside it, and no two
    /// plots share a slot. Compares plots pairwise, so huge grids cost nothing per slot.
    pub(crate) fn validate(&self) -> Result<()> {
        if self.rows == 0 || self.cols == 0 {
            return Err(Error::EmptyGrid {
                rows: self.rows,
                cols: self.cols,
            });
        }
        let overlap = |a: &Range<usize>, b: &Range<usize>| a.start < b.end && b.start < a.end;
        for (j, (rows, cols)) in self.spans.iter().enumerate() {
            if rows.start >= rows.end
                || cols.start >= cols.end
                || rows.end > self.rows
                || cols.end > self.cols
            {
                return Err(Error::InvalidCell {
                    index: j,
                    rows: rows.clone(),
                    cols: cols.clone(),
                    grid: (self.rows, self.cols),
                });
            }
            for (i, (earlier_rows, earlier_cols)) in self.spans[..j].iter().enumerate() {
                if overlap(earlier_rows, rows) && overlap(earlier_cols, cols) {
                    return Err(Error::CellsOverlap {
                        first: i,
                        second: j,
                    });
                }
            }
        }
        Ok(())
    }

    /// (left, top, width, height) in pixels of plot `index`'s cell, rows counted from the top.
    /// Only for a validated figure.
    fn cell_rect(&self, index: usize, width: u32, height: u32) -> (u32, u32, u32, u32) {
        let (rows, cols) = &self.spans[index];
        let left = slot_start(cols.start, self.cols, width);
        let right = slot_start(cols.end, self.cols, width);
        let top = slot_start(rows.start, self.rows, height);
        let bottom = slot_start(rows.end, self.rows, height);
        (left, top, right - left, bottom - top)
    }
}

/// Pixel where slot `index` of `count` slots across `length` pixels starts.
fn slot_start(index: usize, count: usize, length: u32) -> u32 {
    let (base, extra) = (length as usize / count, length as usize % count);
    (index * base + index.min(extra)) as u32 // index <= count, so this is at most `length`
}

/// Rows or columns a plot covers in a [`Figure`]: one index (`1`) or a range (`0..2`, `0..=1`,
/// `1..`, `..2`, `..`). Implemented for `usize` and the standard range types; sealed.
pub trait GridSpan: sealed::Span {}

mod sealed {
    use std::ops::Range;

    pub trait Span {
        /// The range covered in a grid dimension of `len` slots.
        fn resolve(self, len: usize) -> Range<usize>;
    }
}

impl sealed::Span for usize {
    fn resolve(self, _len: usize) -> Range<usize> {
        self..self.saturating_add(1)
    }
}

impl sealed::Span for Range<usize> {
    fn resolve(self, _len: usize) -> Range<usize> {
        self
    }
}

impl sealed::Span for RangeInclusive<usize> {
    fn resolve(self, _len: usize) -> Range<usize> {
        *self.start()..self.end().saturating_add(1)
    }
}

impl sealed::Span for RangeFrom<usize> {
    fn resolve(self, len: usize) -> Range<usize> {
        self.start..len
    }
}

impl sealed::Span for RangeTo<usize> {
    fn resolve(self, _len: usize) -> Range<usize> {
        0..self.end
    }
}

impl sealed::Span for RangeToInclusive<usize> {
    fn resolve(self, _len: usize) -> Range<usize> {
        0..self.end.saturating_add(1)
    }
}

impl sealed::Span for RangeFull {
    fn resolve(self, len: usize) -> Range<usize> {
        0..len
    }
}

impl GridSpan for usize {}
impl GridSpan for Range<usize> {}
impl GridSpan for RangeInclusive<usize> {}
impl GridSpan for RangeFrom<usize> {}
impl GridSpan for RangeTo<usize> {}
impl GridSpan for RangeToInclusive<usize> {}
impl GridSpan for RangeFull {}

#[cfg(test)]
mod tests {
    use super::{sealed::Span, *};

    #[test]
    fn spans_resolve_to_ranges() {
        assert_eq!(2usize.resolve(4), 2..3);
        assert_eq!((1usize..3).resolve(4), 1..3);
        assert_eq!((1usize..=2).resolve(4), 1..3);
        assert_eq!((1usize..).resolve(4), 1..4);
        assert_eq!((..2usize).resolve(4), 0..2);
        assert_eq!((..=2usize).resolve(4), 0..3);
        assert_eq!((..).resolve(4), 0..4);
        // no overflow at the limits; validation rejects these
        assert_eq!((0usize..=usize::MAX).resolve(4), 0..usize::MAX);
        assert_eq!(usize::MAX.resolve(4), usize::MAX..usize::MAX);
    }

    #[test]
    fn plots_keep_the_order_they_were_added_in() {
        // unsuffixed literals and ranges are accepted, as for slice indexing
        let mut fig = Figure::new(2, 2).plot(0, 1, Plot::new().title("b")).plot(
            1,
            ..,
            Plot::new().title("a"),
        );
        assert_eq!(fig.plots()[0].graph().title(), Some("b"));
        fig.plots_mut()[1] = Plot::new().title("c");
        assert_eq!(fig.plots()[1].graph().title(), Some("c"));
        assert_eq!(fig.spans[1], (1..2, 0..2));
    }

    #[test]
    fn slots_share_the_remainder_from_the_first() {
        let starts = |count: usize, length: u32| -> Vec<u32> {
            (0..=count).map(|i| slot_start(i, count, length)).collect()
        };
        assert_eq!(starts(3, 100), [0, 34, 67, 100]);
        assert_eq!(starts(2, 7), [0, 4, 7]);
        assert_eq!(starts(4, 2), [0, 1, 2, 2, 2]);
        assert_eq!(slot_start(1, usize::MAX, 10), 1);
    }

    #[test]
    fn a_spanning_plot_covers_the_union_of_its_slots() {
        let fig = Figure::new(3, 3).plot(1.., 0..2, Plot::new());
        assert_eq!(fig.cell_rect(0, 100, 100), (0, 34, 67, 66));
    }

    #[test]
    fn a_grid_without_rows_or_columns_is_an_error() {
        assert!(matches!(
            Figure::new(0, 2).validate(),
            Err(Error::EmptyGrid { rows: 0, cols: 2 })
        ));
        assert!(matches!(
            Figure::new(2, 0).validate(),
            Err(Error::EmptyGrid { rows: 2, cols: 0 })
        ));
    }

    #[test]
    fn a_plot_outside_the_grid_or_covering_nothing_is_an_error() {
        let fig = Figure::new(2, 2)
            .plot(0, 0, Plot::new())
            .plot(1, 1..3, Plot::new());
        match fig.validate() {
            Err(Error::InvalidCell {
                index: 1,
                rows,
                cols,
                grid: (2, 2),
            }) => {
                assert_eq!((rows, cols), (1..2, 1..3));
            }
            other => panic!("{other:?}"),
        }
        for fig in [
            Figure::new(2, 2).plot(1..1, 0, Plot::new()),
            Figure::new(2, 2).plot(0..=usize::MAX, 0, Plot::new()),
            Figure::new(2, 2).plot(usize::MAX, 0, Plot::new()),
        ] {
            assert!(matches!(
                fig.validate(),
                Err(Error::InvalidCell { index: 0, .. })
            ));
        }
    }

    #[test]
    fn plots_sharing_a_slot_are_an_error() {
        let fig = Figure::new(2, 2)
            .plot(0, 0, Plot::new())
            .plot(1, 1, Plot::new())
            .plot(.., 1, Plot::new()); // covers (0, 1) and (1, 1)
        assert!(matches!(
            fig.validate(),
            Err(Error::CellsOverlap {
                first: 1,
                second: 2
            })
        ));
    }

    #[test]
    fn valid_grids_pass() {
        assert!(Figure::new(2, 2).validate().is_ok()); // no plots: drawn as background
        let fig = Figure::new(2, 2)
            .plot(0, 0, Plot::new())
            .plot(0, 1, Plot::new())
            .plot(1, .., Plot::new());
        assert!(fig.validate().is_ok());
    }

    #[test]
    fn huge_grids_cost_nothing_per_slot() {
        let fig = Figure::new(usize::MAX, usize::MAX)
            .plot(0, 0, Plot::new())
            .plot(1.., 1.., Plot::new());
        assert!(fig.validate().is_ok());
    }

    use crate::{
        plotting::colors,
        terminal::{Terminal, WindowSize},
    };

    fn sine() -> Plot {
        Plot::new().line(
            (0..50)
                .map(|i| (i, (f64::from(i) / 5.0).sin()))
                .collect::<Vec<_>>(),
        )
    }

    /// The `w` x `h` block at (`left`, `top`) of an RGB8 image `width` pixels wide.
    fn block(rgb: &[u8], width: u32, (left, top, w, h): (u32, u32, u32, u32)) -> Vec<u8> {
        (top..top + h)
            .flat_map(|y| {
                let start = ((y * width + left) * 3) as usize;
                rgb[start..start + (w * 3) as usize].to_vec()
            })
            .collect()
    }

    #[test]
    fn a_one_by_one_figure_is_its_plot() {
        let plot = sine().title("one");
        let fig = Figure::new(1, 1).plot(0, 0, plot.clone());
        assert!(fig.render(320, 240).unwrap() == plot.render(320, 240).unwrap());
        let fig = fig.background(colors::WHITE).font_size(20);
        let expected = plot.background(colors::WHITE).font_size(20);
        assert!(fig.render(320, 240).unwrap() == expected.render(320, 240).unwrap());
    }

    #[test]
    fn each_plot_is_drawn_in_its_cell() {
        let left = sine().title("left");
        let right = sine().title("right").background(colors::WHITE);
        let fig = Figure::new(1, 3)
            .plot(0, 0, left.clone())
            .plot(0, 1..3, right.clone());
        let rgb = fig.render(301, 200).unwrap(); // slots of 101, 100 and 100 pixels
        assert!(block(&rgb, 301, (0, 0, 101, 200)) == left.render(101, 200).unwrap());
        assert!(block(&rgb, 301, (101, 0, 200, 200)) == right.render(200, 200).unwrap());
    }

    #[test]
    fn slots_without_a_plot_are_the_figure_background() {
        // cells of 100 x 60: a plot needs more than 50 rows at the default text size
        let rgb = Figure::new(2, 2)
            .plot(0, 0, sine())
            .background(colors::WHITE)
            .render(200, 120)
            .unwrap();
        assert!(
            block(&rgb, 200, (100, 0, 100, 60))
                .iter()
                .all(|&b| b == 255)
        );
        assert!(block(&rgb, 200, (0, 60, 200, 60)).iter().all(|&b| b == 255));
        let empty = Figure::new(2, 3)
            .background(colors::WHITE)
            .render(30, 20)
            .unwrap();
        assert!(empty.len() == 30 * 20 * 3 && empty.iter().all(|&b| b == 255));
    }

    #[test]
    fn a_plots_own_size_is_ignored_in_a_figure() {
        let fig = Figure::new(1, 1).plot(0, 0, sine().size(50, 50));
        assert!(fig.render(320, 240).unwrap() == sine().render(320, 240).unwrap());
    }

    #[test]
    fn cells_without_pixels_fail_with_canvas_too_small() {
        let fig = Figure::new(1, 5).plot(0, 4, sine());
        assert!(matches!(
            fig.render(3, 100),
            Err(Error::CanvasTooSmall { .. })
        ));
        // a huge grid has 1-pixel slots: an error, not a hang or an allocation per slot
        let huge = Figure::new(usize::MAX, usize::MAX).plot(0, 0, sine());
        assert!(matches!(
            huge.render(10, 10),
            Err(Error::CanvasTooSmall { .. })
        ));
    }

    #[test]
    fn an_invalid_grid_draws_nothing() {
        let fig = Figure::new(1, 1).plot(0, 1, sine());
        assert!(matches!(
            fig.render(100, 100),
            Err(Error::InvalidCell { .. })
        ));
    }

    #[test]
    fn show_matches_the_terminal_unless_set() {
        let terminal = Terminal::with_window(WindowSize {
            rows: 50,
            cols: 160,
            x_pix: 1600,
            y_pix: 1700,
            pix_per_row: 34,
            pix_per_col: 10,
        });
        let fig = Figure::new(1, 1);
        assert_eq!(fig.size_in(&terminal), terminal.default_plot_size());
        assert_eq!(fig.font_size_in(&terminal), 28);
        let fig = fig.size(300, 200).font_size(12);
        assert_eq!(
            (fig.size_in(&terminal), fig.font_size_in(&terminal)),
            ((300, 200), 12)
        );
    }

    #[test]
    fn save_png_writes_a_readable_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("figure.png");
        Figure::new(1, 2)
            .plot(0, 0, sine())
            .plot(0, 1, sine())
            .size(320, 240)
            .save_png(&path)
            .unwrap();
        let img = image::open(&path).unwrap();
        assert_eq!((img.width(), img.height()), (320, 240));
    }
}
