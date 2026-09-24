//! Series specifications (`--series`) and style resolution.

use crate::{Result, data::Column};
use rgb::RGB8;
use termplt::plotting::{
    colors, line::LineStyle, marker::MarkerStyle, point::Point, series::Series,
};

pub const DEFAULT_MARKER_SIZE: u32 = 2;
pub const DEFAULT_LINE_THICKNESS: u32 = 0;

/// Colors and marker shapes assigned to series in order when not set explicitly.
const PALETTE: &[(RGB8, Marker)] = &[
    (colors::DODGER_BLUE, Marker::FilledCircle),
    (colors::RED, Marker::HollowCircle),
    (colors::LIME, Marker::FilledSquare),
    (colors::ORANGE, Marker::HollowSquare),
    (colors::CYAN, Marker::FilledCircle),
    (colors::MAGENTA, Marker::HollowCircle),
];

/// Where a series' data comes from.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    Inline(String),
    /// A file path, or "-" for stdin.
    File(String),
}

impl Source {
    pub fn describe(&self) -> String {
        match self {
            Source::Inline(_) => "inline data".to_string(),
            Source::File(path) if path == "-" => "stdin".to_string(),
            Source::File(path) => format!("'{path}'"),
        }
    }
}

/// Style settings; unset fields fall back to broader defaults and then the palette.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Style {
    pub color: Option<String>,
    pub marker: Option<String>,
    pub marker_size: Option<u32>,
    pub marker_color: Option<String>,
    pub line: Option<String>,
    pub line_color: Option<String>,
    pub line_thickness: Option<u32>,
}

impl Style {
    /// Fills unset fields from `defaults`.
    pub fn or(&self, defaults: &Style) -> Style {
        Style {
            color: self.color.clone().or_else(|| defaults.color.clone()),
            marker: self.marker.clone().or_else(|| defaults.marker.clone()),
            marker_size: self.marker_size.or(defaults.marker_size),
            marker_color: self
                .marker_color
                .clone()
                .or_else(|| defaults.marker_color.clone()),
            line: self.line.clone().or_else(|| defaults.line.clone()),
            line_color: self
                .line_color
                .clone()
                .or_else(|| defaults.line_color.clone()),
            line_thickness: self.line_thickness.or(defaults.line_thickness),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeriesSpec {
    pub source: Source,
    pub x: Option<Column>,
    pub y: Option<Column>,
    pub style: Style,
}

impl SeriesSpec {
    pub fn new(source: Source) -> SeriesSpec {
        SeriesSpec {
            source,
            x: None,
            y: None,
            style: Style::default(),
        }
    }
}

const KEYS: &[&str] = &[
    "file",
    "data",
    "x",
    "y",
    "color",
    "marker",
    "marker-size",
    "marker-color",
    "line",
    "line-color",
    "line-thickness",
];

/// Parses a `--series` spec: comma- (or semicolon-) separated `key=value` pairs, e.g.
/// `file=a.csv,y=temp,color=red`. A separator only splits pairs when it is followed by
/// `key=`, so inline data such as `data=(1,2),(3,4)` needs no quoting.
pub fn parse_spec(spec: &str) -> Result<SeriesSpec> {
    let mut source = None;
    let mut x = None;
    let mut y = None;
    let mut style = Style::default();

    for pair in split_pairs(spec) {
        let Some((key, value)) = pair.split_once('=') else {
            return Err(format!("invalid series spec '{spec}': expected key=value pairs").into());
        };
        let key = key.trim().to_ascii_lowercase().replace('_', "-");
        let value = value.trim().to_string();
        let number = |v: &str| {
            v.parse::<u32>()
                .map_err(|_| format!("series key '{key}' needs a non-negative integer, got '{v}'"))
        };
        match key.as_str() {
            "file" | "data" => {
                if source.is_some() {
                    return Err(format!(
                        "series spec '{spec}' has more than one data source (file= or data=)"
                    )
                    .into());
                }
                source = Some(if key == "file" {
                    Source::File(value)
                } else {
                    Source::Inline(value)
                });
            }
            "x" => x = Some(Column::parse(&value)?),
            "y" => y = Some(Column::parse(&value)?),
            "color" => style.color = Some(value),
            "marker" => style.marker = Some(value),
            "marker-size" => style.marker_size = Some(number(&value)?),
            "marker-color" => style.marker_color = Some(value),
            "line" => style.line = Some(value),
            "line-color" => style.line_color = Some(value),
            "line-thickness" => style.line_thickness = Some(number(&value)?),
            _ => {
                let hint = suggest(&key, KEYS)
                    .map(|s| format!(" (did you mean '{s}'?)"))
                    .unwrap_or_default();
                return Err(format!(
                    "unknown series key '{key}'{hint}; valid keys: {}",
                    KEYS.join(", ")
                )
                .into());
            }
        }
    }

    let source = source.ok_or_else(|| {
        format!("series spec '{spec}' needs a data source: file=PATH or data=POINTS")
    })?;
    Ok(SeriesSpec {
        source,
        x,
        y,
        style,
    })
}

/// Splits at ',' or ';' where the text that follows starts with `key=`.
fn split_pairs(spec: &str) -> Vec<&str> {
    let starts_pair = |rest: &str| {
        let rest = rest.trim_start();
        let key_len = rest
            .char_indices()
            .take_while(|(i, c)| c.is_ascii_alphabetic() || (*i > 0 && (*c == '-' || *c == '_')))
            .count();
        key_len > 0 && rest[key_len..].starts_with('=')
    };

    let mut pairs = Vec::new();
    let mut start = 0;
    for (i, c) in spec.char_indices() {
        if (c == ',' || c == ';') && starts_pair(&spec[i + 1..]) {
            pairs.push(&spec[start..i]);
            start = i + 1;
        }
    }
    pairs.push(&spec[start..]);
    pairs.into_iter().filter(|p| !p.trim().is_empty()).collect()
}

/// Returns the candidate closest to `input` if it is plausibly a typo.
pub fn suggest<'a>(input: &str, candidates: &[&'a str]) -> Option<&'a str> {
    candidates
        .iter()
        .map(|c| (edit_distance(input, c), *c))
        .filter(|(d, c)| *d <= 2.max(c.len() / 3))
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Marker {
    None,
    FilledCircle,
    HollowCircle,
    FilledSquare,
    HollowSquare,
}

/// Marker style names accepted on the command line (separators and case are ignored).
pub const MARKER_NAMES: &[(&str, &str)] = &[
    ("filled-circle", "filled circle (also: circle)"),
    ("hollow-circle", "circle outline"),
    ("filled-square", "filled square (also: square)"),
    ("hollow-square", "square outline"),
    ("none", "no markers (line only)"),
];

fn parse_marker(name: &str) -> Result<Marker> {
    let normalized: String = name
        .chars()
        .filter(|c| !matches!(c, '-' | '_' | ' '))
        .collect::<String>()
        .to_ascii_lowercase();
    match normalized.as_str() {
        "filledcircle" | "circle" => Ok(Marker::FilledCircle),
        "hollowcircle" => Ok(Marker::HollowCircle),
        "filledsquare" | "square" => Ok(Marker::FilledSquare),
        "hollowsquare" => Ok(Marker::HollowSquare),
        "none" => Ok(Marker::None),
        _ => Err(format!(
            "unknown marker style '{name}'; valid styles: {}",
            MARKER_NAMES
                .iter()
                .map(|(n, _)| *n)
                .collect::<Vec<_>>()
                .join(", ")
        )
        .into()),
    }
}

pub fn parse_color(name: &str) -> Result<RGB8> {
    colors::parse(name).ok_or_else(|| {
        format!(
            "unknown color '{name}'; use a color name or #RRGGBB (run 'termplt --list-colors' \
             for the names)"
        )
        .into()
    })
}

/// Builds a series from its points and style. `index` selects the palette entry used for
/// anything the style leaves unset.
pub fn build_series(points: &[Point<f64>], style: &Style, index: usize) -> Result<Series<f64>> {
    let (palette_color, palette_marker) = PALETTE[index % PALETTE.len()];

    let color = style.color.as_deref().map(parse_color).transpose()?;
    let marker_color = style
        .marker_color
        .as_deref()
        .map(parse_color)
        .transpose()?
        .or(color)
        .unwrap_or(palette_color);
    let line_color = style
        .line_color
        .as_deref()
        .map(parse_color)
        .transpose()?
        .or(color)
        .unwrap_or(palette_color);

    let size = style.marker_size.unwrap_or(DEFAULT_MARKER_SIZE);
    let marker = match style.marker.as_deref() {
        Some(name) => parse_marker(name)?,
        None => palette_marker,
    };
    let marker_style = match marker {
        Marker::None => MarkerStyle::None,
        Marker::FilledCircle => MarkerStyle::FilledCircle {
            size,
            color: marker_color,
        },
        Marker::HollowCircle => MarkerStyle::HollowCircle {
            size,
            color: marker_color,
        },
        Marker::FilledSquare => MarkerStyle::FilledSquare {
            size,
            color: marker_color,
        },
        Marker::HollowSquare => MarkerStyle::HollowSquare {
            size,
            color: marker_color,
        },
    };

    let thickness = style.line_thickness.unwrap_or(DEFAULT_LINE_THICKNESS);
    let line_style = match style
        .line
        .as_deref()
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        None | Some("solid") => Some(LineStyle::Solid {
            color: line_color,
            thickness,
        }),
        Some("dashed") => Some(LineStyle::Dashed {
            color: line_color,
            thickness,
        }),
        Some("none") => None,
        Some(_) => {
            return Err(format!(
                "unknown line style '{}'; valid styles: solid, dashed, none",
                style.line.as_deref().unwrap_or_default()
            )
            .into());
        }
    };

    let series = Series::new(points).with_marker_style(marker_style);
    Ok(match line_style {
        Some(line_style) => series.with_line_style(line_style),
        None => series,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_with_file_columns_and_style() {
        let spec = parse_spec("file=a.csv, y=temp, x=2, color=red, marker=hollow-square, line=dashed, line_thickness=2")
            .unwrap();
        assert_eq!(spec.source, Source::File("a.csv".into()));
        assert_eq!(spec.x, Some(Column::Index(1)));
        assert_eq!(spec.y, Some(Column::Name("temp".into())));
        assert_eq!(spec.style.color.as_deref(), Some("red"));
        assert_eq!(spec.style.marker.as_deref(), Some("hollow-square"));
        assert_eq!(spec.style.line.as_deref(), Some("dashed"));
        assert_eq!(spec.style.line_thickness, Some(2));
    }

    #[test]
    fn spec_inline_data_keeps_its_commas() {
        let spec = parse_spec("data=(1,2),(3,4),color=#ff0000").unwrap();
        assert_eq!(spec.source, Source::Inline("(1,2),(3,4)".into()));
        assert_eq!(spec.style.color.as_deref(), Some("#ff0000"));

        let spec = parse_spec("color=blue;data=1,2 3,4").unwrap();
        assert_eq!(spec.source, Source::Inline("1,2 3,4".into()));
    }

    #[test]
    fn spec_errors() {
        assert!(parse_spec("color=red").is_err(), "no source");
        assert!(parse_spec("file=a.csv,data=(1,2)").is_err(), "two sources");
        assert!(
            parse_spec("file=a.csv,marker-size=big").is_err(),
            "bad number"
        );
        assert!(parse_spec("a.csv").is_err(), "not key=value");
        let err = parse_spec("file=a.csv,colour=red").unwrap_err().to_string();
        assert!(err.contains("did you mean 'color'"), "{err}");
    }

    #[test]
    fn style_or_prefers_own_values() {
        let own = Style {
            color: Some("red".into()),
            ..Style::default()
        };
        let defaults = Style {
            color: Some("blue".into()),
            marker: Some("none".into()),
            ..Style::default()
        };
        let merged = own.or(&defaults);
        assert_eq!(merged.color.as_deref(), Some("red"));
        assert_eq!(merged.marker.as_deref(), Some("none"));
    }

    #[test]
    fn marker_names_are_lenient() {
        for name in ["FilledCircle", "filled-circle", "filled_circle", "circle"] {
            assert_eq!(parse_marker(name).unwrap(), Marker::FilledCircle, "{name}");
        }
        assert_eq!(parse_marker("None").unwrap(), Marker::None);
        assert!(parse_marker("triangle").is_err());
    }

    fn two_points() -> Vec<Point<f64>> {
        vec![Point::new(1.0, 2.0), Point::new(3.0, 4.0)]
    }

    #[test]
    fn build_series_defaults_come_from_palette() {
        let series = build_series(&two_points(), &Style::default(), 1).unwrap();
        assert_eq!(
            *series.marker_style(),
            MarkerStyle::HollowCircle {
                size: DEFAULT_MARKER_SIZE,
                color: colors::RED
            }
        );
        assert_eq!(
            *series.line_style(),
            Some(LineStyle::Solid {
                color: colors::RED,
                thickness: DEFAULT_LINE_THICKNESS
            })
        );
    }

    #[test]
    fn build_series_color_applies_to_marker_and_line() {
        let style = Style {
            color: Some("#00ff00".into()),
            line_color: Some("blue".into()),
            ..Style::default()
        };
        let series = build_series(&two_points(), &style, 0).unwrap();
        assert_eq!(
            *series.marker_style(),
            MarkerStyle::FilledCircle {
                size: DEFAULT_MARKER_SIZE,
                color: RGB8::new(0, 255, 0)
            }
        );
        assert_eq!(series.line_style().unwrap().color(), colors::BLUE);
    }

    #[test]
    fn build_series_line_and_marker_none() {
        let style = Style {
            marker: Some("none".into()),
            line: Some("None".into()),
            ..Style::default()
        };
        let series = build_series(&two_points(), &style, 0).unwrap();
        assert_eq!(*series.marker_style(), MarkerStyle::None);
        assert!(series.line_style().is_none());
    }

    #[test]
    fn build_series_dashed_line() {
        let style = Style {
            line: Some("dashed".into()),
            line_thickness: Some(1),
            ..Style::default()
        };
        let series = build_series(&two_points(), &style, 0).unwrap();
        assert!(matches!(
            series.line_style(),
            Some(LineStyle::Dashed { thickness: 1, .. })
        ));
    }

    #[test]
    fn build_series_invalid_style_errors() {
        for style in [
            Style {
                color: Some("nope".into()),
                ..Style::default()
            },
            Style {
                line: Some("wavy".into()),
                ..Style::default()
            },
            Style {
                marker: Some("triangle".into()),
                ..Style::default()
            },
        ] {
            assert!(build_series(&two_points(), &style, 0).is_err(), "{style:?}");
        }
    }

    #[test]
    fn suggestions() {
        assert_eq!(suggest("colour", KEYS), Some("color"));
        assert_eq!(suggest("line-thicknes", KEYS), Some("line-thickness"));
        assert_eq!(suggest("zzzzzz", KEYS), None);
    }
}
