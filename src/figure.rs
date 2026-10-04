//! Several plots drawn as one image, on a grid.

use crate::{
    Error, Plot, Result,
    plotting::{colors, font::Font},
};
use rgb::RGB8;
use std::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

/// Several plots drawn as one image, on a grid of equal-size slots. A plot can cover one slot
/// or a block of them.
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

    /// Checks the grid: it has rows and columns, every plot covers slots inside it, and no two
    /// plots share a slot. Compares plots pairwise, so huge grids cost nothing per slot.
    // Task 4 uses this when drawing.
    #[cfg_attr(not(test), expect(dead_code))]
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
    // Task 4 uses this when drawing.
    #[cfg_attr(not(test), expect(dead_code))]
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
// Task 4 uses this when drawing.
#[cfg_attr(not(test), expect(dead_code))]
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
}
