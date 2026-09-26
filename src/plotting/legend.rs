//! The legend: which series it lists, how big it is, where it goes, and its frame.

use super::{
    canvas::LINE_SPACING,
    line::LineStyle,
    marker::MarkerStyle,
    series::Series,
    text::{ELLIPSIS, wrap},
};

/// Where the legend goes inside the plot area. The fixed locations are matplotlib's `loc`
/// values, in the same order; [`LegendLocation::Best`] picks the one covering the least data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LegendLocation {
    /// The fixed location covering the least data, as matplotlib's `loc="best"` picks it.
    #[default]
    Best,
    /// The upper right corner.
    UpperRight,
    /// The upper left corner.
    UpperLeft,
    /// The lower left corner.
    LowerLeft,
    /// The lower right corner.
    LowerRight,
    /// Centered on the right edge: the same place as [`LegendLocation::CenterRight`].
    Right,
    /// Centered on the left edge.
    CenterLeft,
    /// Centered on the right edge.
    CenterRight,
    /// Centered on the bottom edge.
    LowerCenter,
    /// Centered on the top edge.
    UpperCenter,
    /// The middle of the plot.
    Center,
}

/// Space inside the frame, around the entries, in em (matplotlib's `legend.borderpad`).
pub(crate) const BORDER_PAD: f32 = 0.4;
/// Space between entries, in em (`legend.labelspacing`).
pub(crate) const LABEL_SPACING: f32 = 0.5;
/// Width of a sample, in em (`legend.handlelength`).
pub(crate) const HANDLE_LENGTH: f32 = 2.0;
/// Space between a sample and its label, in em (`legend.handletextpad`).
pub(crate) const HANDLE_TEXT_PAD: f32 = 0.8;
/// Space between the frame and the plot's edges, in em (`legend.borderaxespad`).
pub(crate) const BORDER_AXES_PAD: f32 = 0.5;
/// Radius of the frame's corners, in em (matplotlib's `fancybox`).
pub(crate) const CORNER_RADIUS: f32 = 0.2;
/// Most lines a label wraps onto.
pub(crate) const MAX_LABEL_LINES: usize = 2;

/// `fraction` of an em at `size` pixels, in whole pixels.
pub(crate) fn em(fraction: f32, size: u32) -> u32 {
    (fraction * size as f32).round() as u32
}

/// The legend text's size and line metrics, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TextSize {
    /// The em size.
    pub size: u32,
    /// Height of one line box.
    pub line_height: u32,
    /// Rows from the top of a line box to the middle of its digits.
    pub digit_center: u32,
}

/// A labeled series as the legend lists it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Entry {
    pub label: String,
    pub marker: MarkerStyle,
    pub line: Option<LineStyle>,
}

impl Entry {
    /// The entries for `series`: those with a label, in order.
    pub fn from_series(series: &[Series]) -> Vec<Entry> {
        (series.iter())
            .filter_map(|s| {
                Some(Entry {
                    label: s.label()?.to_string(),
                    marker: *s.marker_style(),
                    line: s.line_style().copied(),
                })
            })
            .collect()
    }

    /// How far the sample reaches from its center: the marker's radius or the line's extra
    /// thickness, whichever is larger.
    fn sample_reach(&self) -> u32 {
        (self.marker.size()).max(self.line.map_or(0, |line| line.thickness()))
    }
}

/// A legend laid out with its top-left pixel at (0, 0) and offsets counting down.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Legend {
    pub width: u32,
    pub height: u32,
    /// Left edge of the sample column, which is the same for every entry.
    pub sample_left: u32,
    pub sample_width: u32,
    /// Left edge of the text column.
    pub text_left: u32,
    pub rows: Vec<Row>,
}

/// An entry, or the closing `+N more` row.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Row {
    /// Each line of text with the offset of its top.
    pub text: Vec<(String, u32)>,
    /// `None` for the `+N more` row.
    pub sample: Option<Sample>,
}

/// What an entry's sample shows, and the offset of its center.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Sample {
    pub marker: MarkerStyle,
    pub line: Option<LineStyle>,
    pub center: u32,
}

/// Lays out the legend for `entries` in a plot area of `plot_width` x `plot_height` pixels,
/// measuring text with `measure`. The box is at most half the plot wide and fits inside the
/// plot's pads; labels wrap onto [`MAX_LABEL_LINES`], then end with `…`; entries that don't fit
/// are counted in a closing `+N more` row. `None` when there are no entries, or not even one
/// fits.
pub(crate) fn build(
    entries: &[Entry],
    text: TextSize,
    measure: impl Fn(&str) -> f32,
    plot_width: u32,
    plot_height: u32,
) -> Option<Legend> {
    if entries.is_empty() {
        return None;
    }
    let px = |fraction: f32| em(fraction, text.size);
    let (pad, spacing, axes_pad) = (px(BORDER_PAD), px(LABEL_SPACING), px(BORDER_AXES_PAD));
    let pitch = (text.size as f32 * LINE_SPACING).round() as u32;

    // one sample column for every entry, wide enough for the widest marker or line
    let sample_width = (entries.iter())
        .map(|e| e.sample_reach().saturating_mul(2).saturating_add(1))
        .fold(px(HANDLE_LENGTH), u32::max);
    let max_width = (plot_width / 2).min(plot_width.saturating_sub(2 * axes_pad));
    let max_height = plot_height.saturating_sub(2 * axes_pad);
    let text_left = (pad.saturating_add(sample_width)).saturating_add(px(HANDLE_TEXT_PAD));
    let text_width = max_width.checked_sub(text_left.saturating_add(pad))?;
    if measure(ELLIPSIS) > text_width as f32 {
        return None;
    }

    // each entry's lines, and (height, text top, sample center) within it: the sample's
    // center sits on the text's center line and both fit
    let laid: Vec<(Vec<String>, (u32, u32, u32))> = (entries.iter())
        .map(|entry| {
            let mut lines = wrap(&entry.label, text_width as f32, MAX_LABEL_LINES, &measure);
            if lines.is_empty() {
                // a blank label: the sample with an empty line
                lines.push(String::new());
            }
            let n = lines.len() as u32;
            let text_center = text.digit_center + (n - 1) * pitch / 2;
            let block = text.line_height + (n - 1) * pitch;
            let reach = entry.sample_reach();
            let center = text_center.max(reach);
            let text_top = center - text_center;
            let height = (text_top.saturating_add(block))
                .max(center.saturating_add(reach).saturating_add(1));
            (lines, (height, text_top, center))
        })
        .collect();

    // the first `k` entries, plus a "+N more" row when `more`
    let height_of = |k: usize, more: bool| -> u64 {
        let rows: u64 = laid[..k].iter().map(|(_, (h, _, _))| u64::from(*h)).sum();
        let more_row = if more {
            u64::from(spacing) + u64::from(text.line_height)
        } else {
            0
        };
        2 * u64::from(pad) + rows + u64::from(spacing) * (k as u64 - 1) + more_row
    };
    let fits = |k: usize, more: bool| height_of(k, more) <= u64::from(max_height);
    let shown = if fits(laid.len(), false) {
        laid.len()
    } else {
        (1..laid.len()).rev().find(|&k| fits(k, true))?
    };
    let more = (shown < laid.len()).then(|| format!("+{} more", laid.len() - shown));
    if more
        .as_deref()
        .is_some_and(|m| measure(m) > text_width as f32)
    {
        return None;
    }

    let mut rows = Vec::new();
    let mut top = pad;
    for (entry, (lines, (height, text_top, center))) in entries.iter().zip(laid).take(shown) {
        let text = (lines.into_iter().enumerate())
            .map(|(i, line)| (line, top + text_top + i as u32 * pitch))
            .collect();
        let sample = Sample {
            marker: entry.marker,
            line: entry.line,
            center: top + center,
        };
        rows.push(Row {
            text,
            sample: Some(sample),
        });
        top += height + spacing;
    }
    if let Some(more) = more {
        rows.push(Row {
            text: vec![(more, top)],
            sample: None,
        });
        top += text.line_height + spacing;
    }
    let widest = (rows.iter().flat_map(|row| &row.text))
        .map(|(line, _)| measure(line).ceil() as u32)
        .max()
        .unwrap_or(0);
    Some(Legend {
        width: text_left + widest + pad,
        height: top - spacing + pad,
        sample_left: pad,
        sample_width,
        text_left,
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plotting::colors;

    /// 14 px text in a fake font: 16 px lines, digits centered 8 px down, 7 px per character.
    const TEXT: TextSize = TextSize {
        size: 14,
        line_height: 16,
        digit_center: 8,
    };

    fn measure(text: &str) -> f32 {
        text.chars().count() as f32 * 7.0
    }

    fn entry(label: &str) -> Entry {
        Entry {
            label: label.to_string(),
            marker: MarkerStyle::None,
            line: Some(LineStyle::solid(colors::RED, 0)),
        }
    }

    fn texts(legend: &Legend) -> Vec<Vec<(&str, u32)>> {
        (legend.rows.iter())
            .map(|row| row.text.iter().map(|(l, top)| (l.as_str(), *top)).collect())
            .collect()
    }

    #[test]
    fn entries_are_the_labeled_series_in_order() {
        let series = [
            Series::from(vec![(0, 0)]).with_label("a"),
            Series::from(vec![(0, 0)]),
            Series::from(vec![(0, 0)]).with_label(""),
            Series::from(vec![(0, 0)])
                .with_label("b")
                .with_line_style(LineStyle::dashed(colors::RED, 2)),
        ];
        let entries = Entry::from_series(&series);
        let labels: Vec<&str> = entries.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(labels, ["a", "b"]);
        assert_eq!(entries[1].line, Some(LineStyle::dashed(colors::RED, 2)));
    }

    #[test]
    fn one_entry_uses_matplotlibs_spacing() {
        // pads: border 6, sample 28, sample to text 11
        let legend = build(&[entry("sin")], TEXT, measure, 400, 300).unwrap();
        assert_eq!(
            (legend.width, legend.height),
            (6 + 28 + 11 + 21 + 6, 6 + 16 + 6)
        );
        assert_eq!(
            (legend.sample_left, legend.sample_width, legend.text_left),
            (6, 28, 45)
        );
        assert_eq!(texts(&legend), [vec![("sin", 6)]]);
        // the sample is centered on the digits
        assert_eq!(legend.rows[0].sample.as_ref().unwrap().center, 6 + 8);
    }

    #[test]
    fn entries_are_spaced_and_a_wrapped_one_centers_its_sample() {
        // plot 130 wide: text column 65 - 51 = 14 px, so "a b" wraps
        let legend = build(&[entry("ab"), entry("a b")], TEXT, measure, 130, 300).unwrap();
        // second row: 0.5 em (7) below the first; its lines are 17 px apart
        assert_eq!(
            texts(&legend),
            [vec![("ab", 6)], vec![("a", 29), ("b", 46)]]
        );
        let centers: Vec<u32> = (legend.rows.iter())
            .map(|row| row.sample.as_ref().unwrap().center)
            .collect();
        // halfway between the two lines' digit centers
        assert_eq!(centers, [14, 29 + 16]);
        assert_eq!(
            (legend.width, legend.height),
            (45 + 14 + 6, 6 + 16 + 7 + 33 + 6)
        );
    }

    #[test]
    fn a_big_marker_widens_the_sample_and_heightens_its_entry() {
        let big = Entry {
            marker: MarkerStyle::FilledCircle {
                size: 20,
                color: colors::RED,
            },
            line: None,
            ..entry("big")
        };
        let legend = build(&[big], TEXT, measure, 400, 300).unwrap();
        assert_eq!(legend.sample_width, 41);
        // the text sits on the marker's center, 20 px down
        assert_eq!(texts(&legend), [vec![("big", 6 + 12)]]);
        assert_eq!(legend.rows[0].sample.as_ref().unwrap().center, 6 + 20);
        assert_eq!(legend.height, 6 + 41 + 6);
    }

    #[test]
    fn a_long_label_wraps_at_half_the_plot_then_ends_with_an_ellipsis() {
        let label = "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii";
        let legend = build(&[entry(label)], TEXT, measure, 400, 300).unwrap();
        let lines: Vec<&str> = legend.rows[0]
            .text
            .iter()
            .map(|(l, _)| l.as_str())
            .collect();
        assert_eq!(lines, ["aaaa bbbb cccc dddd", "eeee ffff gggg hhhh…"]);
        assert!(legend.width <= 200);
    }

    #[test]
    fn entries_that_do_not_fit_are_counted() {
        let entries: Vec<Entry> = (0..10).map(|i| entry(&format!("s{i}"))).collect();
        // 106 px of height: three entries and the "+7 more" row
        let legend = build(&entries, TEXT, measure, 400, 120).unwrap();
        assert_eq!(legend.rows.len(), 4);
        assert_eq!(texts(&legend)[3], [("+7 more", 6 + 3 * 23)]);
        assert!(legend.rows[3].sample.is_none());
        assert_eq!(legend.height, 97);
    }

    #[test]
    fn no_legend_when_nothing_fits() {
        let entries: Vec<Entry> = (0..10).map(|i| entry(&format!("s{i}"))).collect();
        // not one entry with the "+N more" row
        assert_eq!(build(&entries, TEXT, measure, 400, 60), None);
        // a single entry needs no such row
        assert!(build(&entries[..1], TEXT, measure, 400, 60).is_some());
        // a text column narrower than the pads
        assert_eq!(build(&entries[..1], TEXT, measure, 100, 300), None);
        // "+7 more" wider than a 14 px text column
        assert_eq!(build(&entries, TEXT, measure, 130, 120), None);
        assert_eq!(build(&[], TEXT, measure, 400, 300), None);
    }

    #[test]
    fn a_blank_label_keeps_its_sample() {
        let legend = build(&[entry(" \n ")], TEXT, measure, 400, 300).unwrap();
        assert_eq!(texts(&legend), [vec![("", 6)]]);
        assert!(legend.rows[0].sample.is_some());
    }
}
