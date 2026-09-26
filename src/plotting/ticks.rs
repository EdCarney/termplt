//! Tick selection and label formatting for axes and grid lines.
//!
//! Ticks are placed at multiples of a "nice" step (1, 2 or 5 × 10^k), following Heckbert,
//! "Nice Numbers for Graph Labels" (Graphics Gems, 1990). All labels on an axis use the same
//! number of decimal places.

/// Default upper bound on the number of ticks per axis.
pub const MAX_TICKS: usize = 10;

/// The minus sign in labels: U+2212, as matplotlib uses by default (`axes.unicode_minus`). It
/// is as wide as `+` and sits at its height, unlike the shorter hyphen.
const MINUS: &str = "\u{2212}";

/// Tolerance (in units of the step) used when deciding whether an end of the range is a tick.
const EPS: f64 = 1e-9;

/// Tick values and their formatted labels for one axis.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AxisTicks {
    pub values: Vec<f64>,
    pub labels: Vec<String>,
}

/// Returns tick values within `[min, max]` spaced by the smallest nice step that yields at most
/// `max_count` ticks, along with that step. Returns `None` for an empty or non-finite range.
pub fn nice_ticks(min: f64, max: f64, max_count: usize) -> Option<(Vec<f64>, f64)> {
    let step = nice_step(min, max, max_count)?;
    let first = (min / step - EPS).ceil() as i64;
    let last = (max / step + EPS).floor() as i64;
    // computing k * step (rather than accumulating) keeps values exact multiples, so the tick
    // at zero is exactly zero
    let values = (first..=last).map(|k| k as f64 * step).collect();
    Some((values, step))
}

fn nice_step(min: f64, max: f64, max_count: usize) -> Option<f64> {
    if !min.is_finite() || !max.is_finite() || max <= min || max_count == 0 {
        return None;
    }

    let span = max - min;
    // start one decade below the smallest step that could satisfy max_count
    let start_exp = (span / max_count as f64).log10().floor() as i32 - 1;
    for exp in start_exp..start_exp + 4 {
        for mantissa in [1.0, 2.0, 5.0] {
            let step = mantissa * 10f64.powi(exp);
            let count = (max / step + EPS).floor() - (min / step - EPS).ceil() + 1.0;
            if count <= max_count as f64 {
                return Some(step);
            }
        }
    }
    None
}

/// Formats tick values with a consistent number of decimal places for the given step. Very
/// large magnitudes and very small steps use scientific notation, unless plain decimals are
/// shorter (e.g. timestamps like `1700000050`, where the mantissa would need many digits).
pub fn format_ticks(values: &[f64], step: f64) -> Vec<String> {
    let magnitude = values.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
    let plain = format_all(values, |v| format!("{:.*}", decimals_for(step), v));
    if magnitude < 1e6 && step >= 1e-4 {
        return plain;
    }
    // enough mantissa digits to distinguish neighbouring ticks (f64 has about 15 significant
    // digits; beyond that the ticks can't be told apart anyway)
    let digits = (magnitude.log10().floor() - step.log10().floor()).clamp(0.0, 15.0) as usize;
    let scientific = format_all(values, |v| format!("{v:.digits$e}"));
    let longest = |labels: &[String]| labels.iter().map(String::len).max().unwrap_or(0);
    if longest(&plain) < longest(&scientific) {
        plain
    } else {
        scientific
    }
}

/// Leading digits an offset must save before it is used (matplotlib's
/// `axes.formatter.offset_threshold`).
const OFFSET_THRESHOLD: i32 = 4;

/// The offset to label an axis over `[min, max]` relative to, or 0 for none. This is
/// matplotlib's `ScalarFormatter` rule: when both ends share at least [`OFFSET_THRESHOLD`]
/// leading digits, those digits become the offset (shown as `+1e15`) and the ticks count up
/// from it. Unlike matplotlib, which looks at the outermost visible ticks, this looks at the
/// range itself, so the offset is known before the ticks are fitted.
pub fn axis_offset(min: f64, max: f64) -> f64 {
    if !min.is_finite() || !max.is_finite() || min >= max || (min <= 0.0 && 0.0 <= max) {
        return 0.0;
    }
    let (abs_min, abs_max) = if min > 0.0 { (min, max) } else { (-max, -min) };
    let oom_max = abs_max.log10().ceil() as i32;
    // the smallest power of ten at which the ends are equal
    let Some(differs) = first_oom(oom_max, |p| floor_div(abs_min, p) != floor_div(abs_max, p))
    else {
        return 0.0;
    };
    let mut oom = differs + 1;
    if (abs_max - abs_min) / pow10(oom) <= 1e-2 {
        // the range straddles a multiple of a large power of ten (relative to the span): use
        // the smallest power of ten at which the ends are at most 1 apart
        let Some(apart) = first_oom(oom_max, |p| {
            floor_div(abs_max, p) - floor_div(abs_min, p) > 1.0
        }) else {
            return 0.0;
        };
        oom = apart + 1;
    }
    let digits = floor_div(abs_max, pow10(oom));
    if !digits.is_finite() || digits < 10f64.powi(OFFSET_THRESHOLD - 1) {
        return 0.0;
    }
    // dividing by an exact power of ten, where matplotlib multiplies by an inexact one, gives
    // the float nearest the decimal offset, so its label has no stray digits
    let offset = if oom < 0 {
        digits / pow10(-oom)
    } else {
        digits * pow10(oom)
    };
    min.signum() * offset
}

fn pow10(oom: i32) -> f64 {
    10f64.powf(oom as f64)
}

/// Counts down from 10^`oom_max` to the first power of ten `p` where `found(p)` holds.
fn first_oom(oom_max: i32, found: impl Fn(f64) -> bool) -> Option<i32> {
    // 1e-323 is the smallest power of ten above zero
    (-323..=oom_max).rev().find(|&oom| found(pow10(oom)))
}

/// `a // b` as Python computes it for positive floats, which matplotlib's rule is written in.
/// It can differ from `(a / b).floor()`: `1.0 // 0.1` is 9 because 0.1 is stored slightly
/// above 0.1.
fn floor_div(a: f64, b: f64) -> f64 {
    let div = (a - a % b) / b;
    let floor = div.floor();
    if div - floor > 0.5 {
        floor + 1.0
    } else {
        floor
    }
}

/// The label for an axis offset in matplotlib's style (`+1e15`, `-1e15`, `+1.7e9`), or `None`
/// for no offset. Where matplotlib rounds to 10 significant digits, this shows every digit the
/// offset has, so the tick values it implies are exact.
pub fn format_offset(offset: f64) -> Option<String> {
    if offset == 0.0 || !offset.is_finite() {
        return None;
    }
    // `{:e}` prints the fewest digits that identify the value, e.g. "1.7e9"
    let label = format!("{offset:e}").replace('-', MINUS);
    let label = label.strip_suffix("e0").unwrap_or(&label);
    Some(if offset > 0.0 {
        format!("+{label}")
    } else {
        label.to_string()
    })
}

fn format_all(values: &[f64], format: impl Fn(f64) -> String) -> Vec<String> {
    values
        .iter()
        .map(|&v| {
            if v == 0.0 {
                "0".to_string()
            } else {
                normalize_negative_zero(format(v)).replace('-', MINUS)
            }
        })
        .collect()
}

/// Smallest number of decimal places that represents `step` exactly (up to rounding noise).
fn decimals_for(step: f64) -> usize {
    (0..=12)
        .find(|&d| {
            let scaled = step * 10f64.powi(d as i32);
            (scaled - scaled.round()).abs() <= 1e-6 * scaled.abs()
        })
        .unwrap_or(12)
}

fn normalize_negative_zero(label: String) -> String {
    match label.strip_prefix('-') {
        Some(rest) if rest.chars().all(|c| c == '0' || c == '.') => rest.to_string(),
        _ => label,
    }
}

/// Chooses the densest ticks (at most [`MAX_TICKS`]) for an axis `length_px` pixels long such
/// that the labels are distinct and `fits(ticks, spacing_px)` holds, where `spacing_px` is the
/// pixel distance between neighbouring ticks. Falls back to the sparsest candidate if none fit.
pub fn fit_ticks(
    min: f64,
    max: f64,
    length_px: f64,
    fits: impl Fn(&AxisTicks, f64) -> bool,
) -> AxisTicks {
    let mut sparsest = AxisTicks::default();
    for max_count in (2..=MAX_TICKS).rev() {
        let Some((values, step)) = nice_ticks(min, max, max_count) else {
            break;
        };
        let labels = format_ticks(&values, step);
        let spacing_px = length_px * step / (max - min);
        let ticks = AxisTicks { values, labels };
        // identical labels (a span too small for the digits shown) would be misleading
        let distinct = ticks.labels.windows(2).all(|w| w[0] != w[1]);
        if distinct && fits(&ticks, spacing_px) {
            return ticks;
        }
        sparsest = ticks;
    }
    sparsest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticks(min: f64, max: f64, max_count: usize) -> Vec<f64> {
        nice_ticks(min, max, max_count).unwrap().0
    }

    #[test]
    fn unit_range_uses_tenths() {
        assert_eq!(
            format_ticks(&ticks(0.0, 1.0, 11), 0.1),
            [
                "0", "0.1", "0.2", "0.3", "0.4", "0.5", "0.6", "0.7", "0.8", "0.9", "1.0"
            ]
        );
    }

    #[test]
    fn step_is_one_two_or_five_times_power_of_ten() {
        for (min, max) in [(0.0, 7.3), (-1.0, 1.0), (3.2, 1234.5), (1e-7, 3e-7)] {
            for max_count in 2..=10 {
                let (values, step) = nice_ticks(min, max, max_count).unwrap();
                assert!(values.len() <= max_count);
                let mantissa = step / 10f64.powf(step.log10().floor());
                assert!(
                    [1.0, 2.0, 5.0].iter().any(|m| (mantissa - m).abs() < 1e-9),
                    "step {step} for {min}..{max}"
                );
                assert!(
                    values
                        .iter()
                        .all(|v| *v >= min - 1e-12 && *v <= max + 1e-12)
                );
            }
        }
    }

    #[test]
    fn zero_is_exact_when_in_range() {
        let values = ticks(-0.99, 0.99, 10);
        assert!(values.contains(&0.0), "{values:?}");
        let labels = format_ticks(&values, 0.2);
        assert!(labels.contains(&"0".to_string()), "{labels:?}");
        assert!(!labels.iter().any(|l| l.contains('e')), "{labels:?}");
    }

    #[test]
    fn labels_share_decimal_places() {
        assert_eq!(
            format_ticks(&[0.0, 0.5, 1.0, 1.5], 0.5),
            ["0", "0.5", "1.0", "1.5"]
        );
        assert_eq!(
            format_ticks(&[-0.25, 0.0, 0.25], 0.25),
            ["−0.25", "0", "0.25"]
        );
        assert_eq!(format_ticks(&[10.0, 20.0], 10.0), ["10", "20"]);
    }

    #[test]
    fn large_and_tiny_values_use_scientific_notation() {
        assert_eq!(
            format_ticks(&[2e6, 2.5e6, 3e6], 5e5),
            ["2.0e6", "2.5e6", "3.0e6"]
        );
        assert_eq!(format_ticks(&[1e-5, 2e-5], 1e-5), ["1e−5", "2e−5"]);
    }

    #[test]
    fn negative_zero_is_normalized() {
        assert_eq!(normalize_negative_zero("-0.00".into()), "0.00");
        assert_eq!(normalize_negative_zero("-0.5".into()), "-0.5");
    }

    #[test]
    fn invalid_ranges_have_no_ticks() {
        assert!(nice_ticks(1.0, 1.0, 5).is_none());
        assert!(nice_ticks(2.0, 1.0, 5).is_none());
        assert!(nice_ticks(0.0, f64::NAN, 5).is_none());
        assert!(nice_ticks(0.0, 1.0, 0).is_none());
    }

    #[test]
    fn fit_ticks_reduces_density_until_labels_fit() {
        // each label needs 50 px; a 200 px axis over 0..10 fits at most 5 ticks (step 2 or 5)
        let fitted = fit_ticks(0.0, 10.0, 200.0, |_, spacing| spacing >= 50.0);
        let spacing = 200.0 * (fitted.values[1] - fitted.values[0]) / 10.0;
        assert!(spacing >= 50.0);
        assert_eq!(fitted.values, [0.0, 5.0, 10.0]);
    }

    #[test]
    fn fit_ticks_falls_back_to_sparsest() {
        let fitted = fit_ticks(0.0, 10.0, 10.0, |_, _| false);
        assert!(!fitted.values.is_empty() && fitted.values.len() <= 2);
    }

    #[test]
    fn large_offsets_get_distinct_plain_labels() {
        let (values, step) = nice_ticks(1_700_000_000.0, 1_700_000_100.0, 3).unwrap();
        assert_eq!(
            format_ticks(&values, step),
            ["1700000000", "1700000050", "1700000100"]
        );
    }

    #[test]
    fn scientific_notation_keeps_enough_digits() {
        // plain would be longer here, so the mantissa must carry the difference
        let labels = format_ticks(&[1.0e20, 1.00000001e20], 1e12);
        assert_eq!(labels, ["1.00000000e20", "1.00000001e20"]);
        assert_eq!(format_ticks(&[2e6, 4e6], 2e6), ["2e6", "4e6"]);
    }

    #[test]
    fn offset_matches_matplotlib() {
        // (min, max, offset) from matplotlib 3.11's ScalarFormatter, given min and max as ticks
        let cases = [
            (1e15, 1e15 + 1.6, 1e15),
            // the view around 1e15 + 0.1 * i, which straddles the offset
            (1e15 - 0.125, 1e15 + 1.625, 1e15),
            (1_700_000_000.0, 1_700_000_100.0, 1_700_000_000.0),
            (1_699_999_995.0, 1_700_000_105.0, 1_700_000_000.0),
            (2000.0, 2024.0, 0.0),
            (100_000.0, 100_050.0, 100_000.0),
            (99_990.0, 100_010.0, 100_000.0),
            (-1.0, 1.0, 0.0),
            (0.0, 10.0, 0.0),
            (-1e15 - 1.6, -1e15, -1e15),
            // Python's float `//` makes 1.0 // 0.1 == 9.0, which is what gives this an offset
            (1.0, 1.001, 1.0),
            (1.2345, 1.2346, 1.234),
            (5.0, 5.001, 0.0),
            (
                1_234_567_890_123_000.0,
                1_234_567_890_123_100.0,
                1_234_567_890_123_000.0,
            ),
            (1e17, 1e17 + 64.0, 1e17),
            (9.5e-301, 1.05e-300, 0.0),
        ];
        for (min, max, offset) in cases {
            assert_eq!(axis_offset(min, max), offset, "{min:?}..{max:?}");
        }
    }

    #[test]
    fn offset_labels() {
        let label = |offset| format_offset(offset);
        assert_eq!(label(1e15).as_deref(), Some("+1e15"));
        assert_eq!(label(1_700_000_000.0).as_deref(), Some("+1.7e9"));
        assert_eq!(label(100_000.0).as_deref(), Some("+1e5"));
        assert_eq!(label(-1e15).as_deref(), Some("−1e15"));
        assert_eq!(label(1.0).as_deref(), Some("+1"));
        assert_eq!(label(1.234).as_deref(), Some("+1.234"));
        assert_eq!(label(0.00125).as_deref(), Some("+1.25e−3"));
        // matplotlib shows 10 digits ("+1.23456789e15"), which misstates every tick by 123000
        assert_eq!(
            label(1_234_567_890_123_000.0).as_deref(),
            Some("+1.234567890123e15")
        );
        assert_eq!(label(0.0), None);
    }

    proptest::proptest! {
        #[test]
        fn ticks_near_a_large_offset_are_evenly_spaced(
            base in -1e18f64..1e18,
            // spans from about 15 representable steps of `base` up to 100 times `base`
            span_exp in -14.5f64..2.0,
        ) {
            let (min, max) = (base, base + base.abs().max(1.0) * 10f64.powf(span_exp));
            let offset = axis_offset(min, max);
            let (lo, hi) = (min - offset, max - offset);
            let ticks = fit_ticks(lo, hi, 1000.0, |_, spacing| spacing >= 40.0);
            let px: Vec<f64> = (ticks.values.iter())
                .map(|v| (v - lo) / (hi - lo) * 1000.0)
                .collect();
            for w in px.windows(3) {
                let gaps = (w[1] - w[0], w[2] - w[1]);
                proptest::prop_assert!(
                    (gaps.0 - gaps.1).abs() < 0.5,
                    "{min:?}..{max:?} offset {offset:?}: pixels {px:?}"
                );
            }
            proptest::prop_assert!(
                ticks.labels.windows(2).all(|l| l[0] != l[1]),
                "{:?}",
                ticks.labels
            );
        }
    }

    #[test]
    fn duplicate_labels_never_fit() {
        let fitted = fit_ticks(0.0, 10.0, 1000.0, |ticks, _| {
            ticks.labels.windows(2).all(|w| w[0] != w[1])
        });
        assert!(fitted.values.len() > 2);
        // a fits check that accepts everything still can't get duplicates past fit_ticks
        let big = 1e17;
        let fitted = fit_ticks(big, big + 64.0, 1000.0, |_, _| true);
        assert!(
            fitted.labels.windows(2).all(|w| w[0] != w[1]),
            "{:?}",
            fitted.labels
        );
    }
}
