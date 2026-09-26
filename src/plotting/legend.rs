//! The legend: which series it lists, how big it is, where it goes, and its frame.

use super::{
    canvas::LINE_SPACING,
    limits::Limits,
    line::LineStyle,
    marker::MarkerStyle,
    point::Point,
    series::Series,
    text::{ELLIPSIS, wrap},
};
use rgb::RGB8;

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

impl LegendLocation {
    /// The fixed locations in matplotlib's order (its `loc` codes 1 to 10), the order "best"
    /// tries them in.
    pub(crate) const FIXED: [LegendLocation; 10] = [
        LegendLocation::UpperRight,
        LegendLocation::UpperLeft,
        LegendLocation::LowerLeft,
        LegendLocation::LowerRight,
        LegendLocation::Right,
        LegendLocation::CenterLeft,
        LegendLocation::CenterRight,
        LegendLocation::LowerCenter,
        LegendLocation::UpperCenter,
        LegendLocation::Center,
    ];
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
/// Opacity of the frame, out of 255 (matplotlib's `legend.framealpha`, 0.8).
pub(crate) const FRAME_ALPHA: u8 = 204;

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

    /// The narrowest sample that shows this entry: the marker whole, and the line's round ends
    /// with `run` pixels of straight line between them, so a thick line still reads as a line.
    fn min_sample_width(&self, run: u32) -> u32 {
        let diameter = |reach: u32| reach.saturating_mul(2).saturating_add(1);
        let line = (self.line).map_or(0, |line| diameter(line.thickness()).saturating_add(run));
        diameter(self.marker.size()).max(line)
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

    // one sample column for every entry, wide enough for the widest marker, and for 1 em of
    // straight line between a thick line's round ends
    let sample_width = (entries.iter())
        .map(|e| e.min_sample_width(px(1.0)))
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

/// The legend's box at `location`: `width` x `height` pixels, `pad` pixels in from the edges
/// of `plot` (inclusive pixel bounds, rows counting up). A centered box that can't sit exactly
/// in the middle moves half a pixel toward the lower left. `Best` gives the upper right.
pub(crate) fn candidate(
    location: LegendLocation,
    plot: &Limits<u32>,
    width: u32,
    height: u32,
    pad: u32,
) -> Limits<u32> {
    use LegendLocation::*;
    let (min, max) = (*plot.min(), *plot.max());
    let (plot_w, plot_h) = (max.x - min.x + 1, max.y - min.y + 1);
    let left = match location {
        UpperLeft | LowerLeft | CenterLeft => min.x + pad,
        LowerCenter | UpperCenter | Center => min.x + plot_w.saturating_sub(width) / 2,
        Best | UpperRight | LowerRight | Right | CenterRight => {
            (max.x + 1).saturating_sub(pad + width)
        }
    };
    let bottom = match location {
        LowerLeft | LowerRight | LowerCenter => min.y + pad,
        Right | CenterLeft | CenterRight | Center => min.y + plot_h.saturating_sub(height) / 2,
        Best | UpperRight | UpperLeft | UpperCenter => (max.y + 1).saturating_sub(pad + height),
    };
    Limits::new(
        Point::new(left, bottom),
        Point::new(left + width.max(1) - 1, bottom + height.max(1) - 1),
    )
}

/// Index of the candidate box covering the least data, as matplotlib's `loc="best"` picks it
/// (`Legend._find_best_position`): the first that covers nothing, else the lowest score, a tie
/// going to the earlier box. `series` are in canvas pixels.
pub(crate) fn best(candidates: &[Limits<u32>], series: &[Series]) -> usize {
    let mut lowest = (usize::MAX, 0);
    for (i, candidate) in candidates.iter().enumerate() {
        let score = badness(candidate, series);
        if score == 0 {
            return i;
        }
        if score < lowest.0 {
            lowest = (score, i);
        }
    }
    lowest.1
}

/// matplotlib's score for a legend box: every series' points strictly inside it, plus one for
/// each line whose drawn path touches it. Marker sizes are ignored, as in matplotlib.
fn badness(bbox: &Limits<u32>, series: &[Series]) -> usize {
    let rect = Rect::new(bbox);
    let finite = |p: &Point<f64>| p.x.is_finite() && p.y.is_finite();
    (series.iter())
        .map(|s| {
            let has_line = s.line_style().is_some();
            if !has_line && s.marker_style().color().is_none() {
                return 0; // it draws nothing
            }
            let inside = s.data().iter().filter(|p| rect.contains(p)).count();
            // segments with a non-finite end aren't drawn
            let touches = has_line
                && (s.data().windows(2))
                    .any(|w| finite(&w[0]) && finite(&w[1]) && rect.touches(w[0], w[1]));
            inside + usize::from(touches)
        })
        .sum()
}

/// A box of pixels as a continuous rectangle: pixel `x` covers `x` to `x + 1`.
struct Rect {
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
}

impl Rect {
    fn new(bbox: &Limits<u32>) -> Rect {
        Rect {
            x0: f64::from(bbox.min().x),
            x1: f64::from(bbox.max().x) + 1.0,
            y0: f64::from(bbox.min().y),
            y1: f64::from(bbox.max().y) + 1.0,
        }
    }

    /// Whether `p` is strictly inside; never for a NaN or infinite point.
    fn contains(&self, p: &Point<f64>) -> bool {
        self.x0 < p.x && p.x < self.x1 && self.y0 < p.y && p.y < self.y1
    }

    /// Whether the segment from `a` to `b` touches the rectangle, edges included
    /// (Liang–Barsky clipping).
    fn touches(&self, a: Point<f64>, b: Point<f64>) -> bool {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
        for (p, q) in [
            (-dx, a.x - self.x0),
            (dx, self.x1 - a.x),
            (-dy, a.y - self.y0),
            (dy, self.y1 - a.y),
        ] {
            if p == 0.0 {
                // parallel to this edge: outside it means no contact
                if q < 0.0 {
                    return false;
                }
            } else {
                let t = q / p;
                if p < 0.0 {
                    t0 = t0.max(t);
                } else {
                    t1 = t1.min(t);
                }
                if t0 > t1 {
                    return false;
                }
            }
        }
        true
    }
}

/// How much of each pixel of a `width` x `height` box the frame covers, 0 to 255, row-major
/// from the top: the rounded rectangle's inside (`fill`) and its 1 px edge (`edge`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Frame {
    pub fill: Vec<u8>,
    pub edge: Vec<u8>,
}

/// The frame of a `width` x `height` legend with corners of `radius` pixels, anti-aliased from
/// the distance of each pixel's center to the rounded rectangle. Only `sqrt` is used, which is
/// exact in IEEE arithmetic, so every platform gets the same bytes.
pub(crate) fn frame(width: u32, height: u32, radius: u32) -> Frame {
    let (half_w, half_h) = (width as f32 / 2.0, height as f32 / 2.0);
    let radius = (radius as f32).min(half_w).min(half_h);
    // coverage of a rounded rectangle centered on the origin, at (x, y)
    let coverage = |x: f32, y: f32, half_w: f32, half_h: f32, r: f32| {
        let (qx, qy) = (x.abs() - (half_w - r), y.abs() - (half_h - r));
        let (ox, oy) = (qx.max(0.0), qy.max(0.0));
        let distance = (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r;
        (0.5 - distance).clamp(0.0, 1.0)
    };
    let inner_radius = (radius - 1.0).max(0.0);
    let to_byte = |c: f32| (c * 255.0).round() as u8;
    let pixels = width as usize * height as usize;
    let (mut fill, mut edge) = (Vec::with_capacity(pixels), Vec::with_capacity(pixels));
    for row in 0..height {
        for col in 0..width {
            let (x, y) = (col as f32 + 0.5 - half_w, row as f32 + 0.5 - half_h);
            let outer = coverage(x, y, half_w, half_h, radius);
            let inner = coverage(x, y, half_w - 1.0, half_h - 1.0, inner_radius);
            fill.push(to_byte(inner));
            edge.push(to_byte((outer - inner).max(0.0)));
        }
    }
    Frame { fill, edge }
}

/// The frame's edge color: 80% `background` and 20% `text`, per channel in sRGB, rounded.
/// Black text on white gives matplotlib's `0.8` gray.
pub(crate) fn edge_color(background: RGB8, text: RGB8) -> RGB8 {
    let mix = |b: u8, t: u8| ((4 * u32::from(b) + u32::from(t) + 2) / 5) as u8;
    RGB8::new(
        mix(background.r, text.r),
        mix(background.g, text.g),
        mix(background.b, text.b),
    )
}

/// `coverage` scaled by the frame's opacity, rounded.
pub(crate) fn alpha(coverage: u8) -> u8 {
    ((u32::from(coverage) * u32::from(FRAME_ALPHA) + 127) / 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plotting::{colors, limits::Limits, point::Point};

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
    fn a_thick_line_keeps_a_straight_run_of_one_em() {
        let thick = Entry {
            line: Some(LineStyle::solid(colors::RED, 20)),
            ..entry("thick")
        };
        let legend = build(&[thick], TEXT, measure, 400, 300).unwrap();
        // the round ends take 2 × 20 + 1 px; 1 em (14 px) of straight line stays between them
        assert_eq!(legend.sample_width, 41 + 14);
        // thin lines keep matplotlib's 2 em
        let thin = build(&[entry("thin")], TEXT, measure, 400, 300).unwrap();
        assert_eq!(thin.sample_width, 28);
    }

    #[test]
    fn a_blank_label_keeps_its_sample() {
        let legend = build(&[entry(" \n ")], TEXT, measure, 400, 300).unwrap();
        assert_eq!(texts(&legend), [vec![("", 6)]]);
        assert!(legend.rows[0].sample.is_some());
    }

    #[test]
    fn fixed_locations_follow_matplotlibs_codes() {
        use LegendLocation::*;
        assert_eq!(
            LegendLocation::FIXED,
            [
                UpperRight,
                UpperLeft,
                LowerLeft,
                LowerRight,
                Right,
                CenterLeft,
                CenterRight,
                LowerCenter,
                UpperCenter,
                Center
            ]
        );
    }

    #[test]
    fn candidates_sit_inside_the_pads() {
        use LegendLocation::*;
        // a 200 x 100 plot area; a 60 x 30 legend; a 7 px pad
        let plot = Limits::new(Point::new(10, 20), Point::new(209, 119));
        let expected = [
            (UpperRight, (143, 83), (202, 112)),
            (UpperLeft, (17, 83), (76, 112)),
            (LowerLeft, (17, 27), (76, 56)),
            (LowerRight, (143, 27), (202, 56)),
            (Right, (143, 55), (202, 84)),
            (CenterLeft, (17, 55), (76, 84)),
            (CenterRight, (143, 55), (202, 84)),
            (LowerCenter, (80, 27), (139, 56)),
            (UpperCenter, (80, 83), (139, 112)),
            (Center, (80, 55), (139, 84)),
        ];
        for (location, min, max) in expected {
            let bbox = candidate(location, &plot, 60, 30, 7);
            assert_eq!(
                (*bbox.min(), *bbox.max()),
                (Point::new(min.0, min.1), Point::new(max.0, max.1)),
                "{location:?}"
            );
        }
    }

    #[test]
    fn an_uneven_center_moves_toward_the_lower_left() {
        // 201 x 101: the exact middle is half a pixel right of and above (80, 55)
        let plot = Limits::new(Point::new(10, 20), Point::new(210, 120));
        let bbox = candidate(LegendLocation::Center, &plot, 60, 30, 7);
        assert_eq!(*bbox.min(), Point::new(80, 55));
    }

    /// A marker-only series.
    fn scatter(points: &[(f64, f64)]) -> Series {
        Series::from(points.to_vec())
    }

    /// A line-only series.
    fn line(points: &[(f64, f64)]) -> Series {
        Series::from(points.to_vec())
            .with_marker_style(MarkerStyle::None)
            .with_line_style(LineStyle::default())
    }

    /// Pixels 10 to 19 each way, so the box spans 10.0 to 20.0.
    fn bbox() -> Limits<u32> {
        Limits::new(Point::new(10, 10), Point::new(19, 19))
    }

    #[test]
    fn points_strictly_inside_count() {
        let points = scatter(&[
            (15.0, 15.0),
            (12.0, 19.5),
            (10.0, 15.0), // on the left edge
            (20.0, 15.0), // on the right edge
            (25.0, 15.0),
        ]);
        assert_eq!(badness(&bbox(), &[points]), 2);
    }

    #[test]
    fn a_line_touching_the_box_counts_once() {
        // through the box, with no point inside
        assert_eq!(badness(&bbox(), &[line(&[(0.0, 15.0), (30.0, 15.0)])]), 1);
        // inside: its two points and its path
        assert_eq!(badness(&bbox(), &[line(&[(12.0, 12.0), (18.0, 18.0)])]), 3);
        // along the top edge
        assert_eq!(badness(&bbox(), &[line(&[(0.0, 20.0), (30.0, 20.0)])]), 1);
        // past it
        assert_eq!(badness(&bbox(), &[line(&[(0.0, 25.0), (30.0, 25.0)])]), 0);
    }

    #[test]
    fn scatter_points_have_no_path() {
        assert_eq!(
            badness(&bbox(), &[scatter(&[(0.0, 15.0), (30.0, 15.0)])]),
            0
        );
    }

    #[test]
    fn segments_at_a_gap_are_skipped() {
        let gap = line(&[(0.0, 15.0), (f64::NAN, f64::NAN), (30.0, 15.0)]);
        assert_eq!(badness(&bbox(), &[gap]), 0);
    }

    #[test]
    fn a_series_that_draws_nothing_scores_nothing() {
        let invisible = scatter(&[(15.0, 15.0)]).with_marker_style(MarkerStyle::None);
        assert_eq!(badness(&bbox(), &[invisible]), 0);
    }

    #[test]
    fn best_takes_the_first_empty_box_else_the_lowest_score() {
        let boxes = [
            Limits::new(Point::new(0, 0), Point::new(9, 9)),
            Limits::new(Point::new(20, 0), Point::new(29, 9)),
            Limits::new(Point::new(40, 0), Point::new(49, 9)),
        ];
        let at = |x: f64| scatter(&[(x, 5.0)]);
        // the first two are covered
        assert_eq!(best(&boxes, &[at(5.0), at(25.0)]), 2);
        // the second is the first empty one
        assert_eq!(best(&boxes, &[at(5.0)]), 1);
        // none is empty: scores 2, 1, 1, and the tie goes to the earlier box
        assert_eq!(best(&boxes, &[at(5.0), at(6.0), at(25.0), at(45.0)]), 1);
    }

    #[test]
    fn a_square_frame_has_a_one_pixel_edge() {
        let frame = frame(10, 6, 0);
        for y in 0..6 {
            for x in 0..10 {
                let i = y * 10 + x;
                let border = x == 0 || y == 0 || x == 9 || y == 5;
                assert_eq!(
                    frame.edge[i],
                    if border { 255 } else { 0 },
                    "edge ({x}, {y})"
                );
                assert_eq!(
                    frame.fill[i],
                    if border { 0 } else { 255 },
                    "fill ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn rounded_corners_leave_the_corner_pixel_out() {
        let frame = frame(20, 10, 3);
        let at = |v: &[u8], x: usize, y: usize| v[y * 20 + x];
        assert_eq!((at(&frame.edge, 0, 0), at(&frame.fill, 0, 0)), (0, 0));
        // the middle of the top edge is all edge; the middle is all fill
        assert_eq!((at(&frame.edge, 10, 0), at(&frame.fill, 10, 0)), (255, 0));
        assert_eq!((at(&frame.edge, 10, 5), at(&frame.fill, 10, 5)), (0, 255));
        // a pixel on the curve is partly covered
        let curve = at(&frame.edge, 1, 0);
        assert!(0 < curve && curve < 255, "{curve}");
        // the corners match
        assert_eq!(at(&frame.edge, 1, 0), at(&frame.edge, 18, 9));
    }

    #[test]
    fn the_edge_is_a_fifth_of_the_way_to_the_text_color() {
        // matplotlib's 0.8 gray for black text on white
        assert_eq!(
            edge_color(colors::WHITE, colors::BLACK),
            RGB8::new(204, 204, 204)
        );
        assert_eq!(
            edge_color(colors::BLACK, colors::WHITE),
            RGB8::new(51, 51, 51)
        );
        assert_eq!(
            edge_color(RGB8::new(0, 0, 128), colors::WHITE),
            RGB8::new(51, 51, 153)
        );
    }

    #[test]
    fn the_frame_is_80_percent_opaque() {
        assert_eq!(alpha(255), FRAME_ALPHA);
        assert_eq!(alpha(0), 0);
        assert_eq!(alpha(128), 102);
    }
}
