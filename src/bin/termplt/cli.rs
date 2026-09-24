//! Command-line arguments.

use clap::{
    Parser, ValueHint,
    builder::{PossibleValue, PossibleValuesParser, TypedValueParser},
    error::ErrorKind,
};
use clap_complete::Shell;
use std::ffi::OsStr;
use std::path::PathBuf;
use termplt::plotting::colors;

const EXAMPLES: &str = "\
Examples:
  termplt data.csv
  seq 0 10 | awk '{print $1, $1*$1}' | termplt
  termplt --data \"(1,1),(2,4),(3,9)\"
  termplt sensors.csv -x time -y temp,humidity
  termplt -s \"file=a.csv,color=red\" -s \"file=b.csv,color=#1e90ff,marker=none,line=dashed\"
  termplt data.csv --xlim 0,10 --ylim -1,1 -o plot.png

Series specs (-s/--series) are comma-separated key=value pairs:
  file=PATH | data=POINTS   the data for the series ('-' reads stdin); exactly one is required
  x=COL, y=COL              columns to plot (defaults to --x-col/--y-col)
  color, marker, marker-size, marker-color, line, line-color, line-thickness
                            style for this series (defaults to the matching options)

Columns (COL) are header names or 1-based indices; x=index plots y against the row number.
A file with a single column is plotted against the row number.";

#[derive(Parser, Debug)]
#[command(
    name = "termplt",
    version,
    about = "Plot data in a terminal that supports the Kitty graphics protocol",
    after_help = EXAMPLES
)]
pub struct Cli {
    /// Data files (CSV, TSV or whitespace-delimited); '-' reads stdin. Piped stdin is read
    /// automatically when no other data is given
    #[arg(value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub files: Vec<String>,

    /// Inline points, e.g. "(1,2),(3,4)" or "1,2 3,4" (repeatable)
    #[arg(short, long, value_name = "POINTS", allow_hyphen_values = true)]
    pub data: Vec<String>,

    /// A series with its own data and style, e.g. "file=a.csv,y=temp,color=red" (repeatable;
    /// see below)
    #[arg(short, long, value_name = "SPEC", allow_hyphen_values = true)]
    pub series: Vec<String>,

    /// Column for x values in data files: header name, 1-based index, or "index" for the row
    /// number [default: 1]
    #[arg(short = 'x', long, value_name = "COL", help_heading = "Columns")]
    pub x_col: Option<String>,

    /// Column(s) for y values; separate several with commas to plot one series per column
    /// [default: 2]
    #[arg(
        short = 'y',
        long,
        value_name = "COLS",
        value_delimiter = ',',
        help_heading = "Columns"
    )]
    pub y_col: Vec<String>,

    /// Color for markers and lines of every series (name or #RRGGBB); cycles by default
    #[arg(short, long, value_name = "COLOR", value_parser = ColorParser, hide_possible_values = true, help_heading = "Style")]
    pub color: Option<String>,

    /// Marker style
    #[arg(
        long,
        value_name = "STYLE",
        value_parser = marker_values(),
        ignore_case = true,
        alias = "marker-style",
        alias = "marker_style",
        help_heading = "Style"
    )]
    pub marker: Option<String>,

    /// Marker radius in pixels [default: 2]
    #[arg(long, value_name = "PX", alias = "marker_size", help_heading = "Style")]
    pub marker_size: Option<u32>,

    /// Marker color (overrides --color)
    #[arg(
        long,
        value_name = "COLOR",
        value_parser = ColorParser,
        hide_possible_values = true,
        alias = "marker_color",
        help_heading = "Style"
    )]
    pub marker_color: Option<String>,

    /// Line style; none draws a scatter plot [default: solid]
    #[arg(
        long,
        value_name = "STYLE",
        value_parser = PossibleValuesParser::new(["solid", "dashed", "none"]),
        ignore_case = true,
        alias = "line-style",
        alias = "line_style",
        help_heading = "Style"
    )]
    pub line: Option<String>,

    /// Line color (overrides --color)
    #[arg(
        long,
        value_name = "COLOR",
        value_parser = ColorParser,
        hide_possible_values = true,
        alias = "line_color",
        help_heading = "Style"
    )]
    pub line_color: Option<String>,

    /// Line thickness in pixels [default: 0]
    #[arg(
        long,
        value_name = "PX",
        alias = "line_thickness",
        help_heading = "Style"
    )]
    pub line_thickness: Option<u32>,

    /// X axis limits; points outside are not drawn
    #[arg(long, value_name = "MIN,MAX", allow_hyphen_values = true, value_parser = parse_range, help_heading = "Plot")]
    pub xlim: Option<(f64, f64)>,

    /// Y axis limits; points outside are not drawn
    #[arg(long, value_name = "MIN,MAX", allow_hyphen_values = true, value_parser = parse_range, help_heading = "Plot")]
    pub ylim: Option<(f64, f64)>,

    /// Image width in pixels [default: fits the terminal, or 800 with --output]
    #[arg(long, value_name = "PX", value_parser = clap::value_parser!(u32).range(1..), help_heading = "Plot")]
    pub width: Option<u32>,

    /// Image height in pixels [default: 60% of the terminal, or 600 with --output]
    #[arg(long, value_name = "PX", value_parser = clap::value_parser!(u32).range(1..), help_heading = "Plot")]
    pub height: Option<u32>,

    /// Background color
    #[arg(
        long,
        value_name = "COLOR",
        value_parser = ColorParser,
        hide_possible_values = true,
        default_value = "black",
        help_heading = "Plot"
    )]
    pub bg: String,

    /// Hide grid lines
    #[arg(long, help_heading = "Plot")]
    pub no_grid: bool,

    /// Write the plot to a PNG file (e.g. plot.png) instead of displaying it; no terminal is
    /// needed
    #[arg(short, long, value_name = "FILE", value_hint = ValueHint::FilePath, help_heading = "Plot")]
    pub output: Option<PathBuf>,

    /// List the available color names
    #[arg(long)]
    pub list_colors: bool,

    /// List the available marker styles
    #[arg(long)]
    pub list_markers: bool,

    /// Print debug information (terminal size, canvas, plot area) to stderr
    #[arg(short, long)]
    pub verbose: bool,

    /// Same as FILE (kept for compatibility with earlier versions)
    #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath, alias = "data_file", hide = true)]
    pub data_file: Vec<String>,

    /// Print a shell completion script and exit, e.g. `termplt --completions zsh > _termplt`
    #[arg(long, value_name = "SHELL")]
    pub completions: Option<Shell>,
}

/// Accepts color names (ignoring case and separators) and hex colors. The color names are
/// offered as possible values so shells can complete them; hex values are accepted too, so
/// the names are hidden from `--help` rather than enforced as the only choices.
#[derive(Clone)]
struct ColorParser;

impl TypedValueParser for ColorParser {
    type Value = String;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &OsStr,
    ) -> Result<String, clap::Error> {
        let value = value.to_string_lossy();
        if colors::parse(&value).is_some() {
            return Ok(value.into_owned());
        }
        let flag = arg
            .and_then(|a| a.get_long())
            .map(|l| format!(" for '--{l}'"))
            .unwrap_or_default();
        Err(clap::Error::raw(
            ErrorKind::InvalidValue,
            format!(
                "unknown color '{value}'{flag}; use a color name or #RRGGBB (run 'termplt \
                 --list-colors' for the names)\n"
            ),
        )
        .with_cmd(cmd))
    }

    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        Some(Box::new(colors::all_names().iter().map(|(name, _)| {
            PossibleValue::new(color_display_name(name))
        })))
    }
}

/// The spelling used in help, completions and `--list-colors`: lowercase with hyphens.
pub fn color_display_name(name: &str) -> String {
    name.to_ascii_lowercase().replace('_', "-")
}

/// Marker styles; the aliases keep earlier spellings such as FilledCircle working (clap also
/// ignores case) without cluttering help and completions.
fn marker_values() -> PossibleValuesParser {
    PossibleValuesParser::new([
        PossibleValue::new("filled-circle").aliases(["filledcircle", "filled_circle", "circle"]),
        PossibleValue::new("hollow-circle").aliases(["hollowcircle", "hollow_circle"]),
        PossibleValue::new("filled-square").aliases(["filledsquare", "filled_square", "square"]),
        PossibleValue::new("hollow-square").aliases(["hollowsquare", "hollow_square"]),
        PossibleValue::new("none").help("no markers (line only)"),
    ])
}

/// Parses "MIN,MAX" (or "MIN:MAX") into an increasing pair of finite numbers.
fn parse_range(value: &str) -> Result<(f64, f64), String> {
    let (min, max) = value
        .split_once([',', ':'])
        .ok_or_else(|| format!("expected MIN,MAX, got '{value}'"))?;
    let parse = |s: &str| {
        s.trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .ok_or_else(|| format!("'{}' is not a finite number", s.trim()))
    };
    let (min, max) = (parse(min)?, parse(max)?);
    if min >= max {
        return Err(format!("MIN must be less than MAX, got {min},{max}"));
    }
    Ok((min, max))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("termplt").chain(args.iter().copied()))
    }

    #[test]
    fn command_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn files_data_and_series_are_collected() {
        let cli = parse(&[
            "a.csv",
            "-",
            "--data",
            "(1,2),(3,4)",
            "-s",
            "file=b.csv,color=red",
        ])
        .unwrap();
        assert_eq!(cli.files, ["a.csv", "-"]);
        assert_eq!(cli.data, ["(1,2),(3,4)"]);
        assert_eq!(cli.series, ["file=b.csv,color=red"]);
    }

    #[test]
    fn y_columns_split_on_commas() {
        let cli = parse(&["a.csv", "-y", "temp,humidity", "-x", "time"]).unwrap();
        assert_eq!(cli.y_col, ["temp", "humidity"]);
        assert_eq!(cli.x_col.as_deref(), Some("time"));
    }

    #[test]
    fn inline_data_may_start_with_a_minus_sign() {
        let cli = parse(&["--data", "-1,2 3,4"]).unwrap();
        assert_eq!(cli.data, ["-1,2 3,4"]);
    }

    #[test]
    fn legacy_flag_spellings_still_work() {
        let cli = parse(&[
            "--data_file",
            "a.csv",
            "--marker_style",
            "None",
            "--line_thickness",
            "2",
            "--marker_color",
            "Red",
        ])
        .unwrap();
        assert_eq!(cli.data_file, ["a.csv"]);
        assert_eq!(cli.marker.as_deref(), Some("None"));
        assert_eq!(cli.line_thickness, Some(2));
        assert_eq!(cli.marker_color.as_deref(), Some("Red"));
    }

    #[test]
    fn limits_accept_negative_values() {
        let cli = parse(&["a.csv", "--xlim", "-5,5", "--ylim", "-1:0.5"]).unwrap();
        assert_eq!(cli.xlim, Some((-5.0, 5.0)));
        assert_eq!(cli.ylim, Some((-1.0, 0.5)));
    }

    #[test]
    fn invalid_limits_are_rejected() {
        assert!(parse(&["--xlim", "5,1"]).is_err());
        assert!(parse(&["--xlim", "5"]).is_err());
        assert!(parse(&["--xlim", "0,inf"]).is_err());
    }

    #[test]
    fn zero_width_is_rejected() {
        assert!(parse(&["--width", "0"]).is_err());
    }

    #[test]
    fn marker_and_line_accept_known_styles_in_any_spelling() {
        // clap validates the spelling; the value is passed through unchanged and normalized
        // when the series style is built
        for given in [
            "FilledCircle",
            "filled_circle",
            "circle",
            "HOLLOW-SQUARE",
            "None",
        ] {
            let cli = parse(&["a.csv", "--marker", given]).unwrap();
            assert_eq!(cli.marker.as_deref(), Some(given));
        }
        assert!(parse(&["a.csv", "--line", "Dashed"]).is_ok());
    }

    #[test]
    fn unknown_marker_and_line_styles_list_possible_values() {
        let err = parse(&["--marker", "triangle"]).unwrap_err().to_string();
        assert!(err.contains("filled-circle"), "{err}");
        let err = parse(&["--line", "wavy"]).unwrap_err().to_string();
        assert!(err.contains("dashed"), "{err}");
    }

    #[test]
    fn completion_scripts_include_flags_and_values() {
        for shell in [
            Shell::Bash,
            Shell::Zsh,
            Shell::Fish,
            Shell::PowerShell,
            Shell::Elvish,
        ] {
            let mut script = Vec::new();
            clap_complete::generate(shell, &mut Cli::command(), "termplt", &mut script);
            let script = String::from_utf8(script).unwrap();
            // fish writes long flags as `-l xlim`, the others as `--xlim`
            for expected in ["xlim", "series", "completions"] {
                assert!(script.contains(expected), "{shell}: missing {expected}");
            }
            // possible values are listed where the shell's format supports it
            if !matches!(shell, Shell::PowerShell | Shell::Elvish) {
                assert!(
                    script.contains("hollow-square"),
                    "{shell}: missing marker values"
                );
            }
            // hidden aliases stay out of completions
            assert!(!script.contains("filledcircle"), "{shell}: leaked alias");
        }
    }

    #[test]
    fn colors_accept_names_and_hex() {
        let cli = parse(&[
            "a.csv",
            "-c",
            "DarkRed",
            "--line-color",
            "#1e90ff",
            "--bg",
            "white",
        ])
        .unwrap();
        assert_eq!(cli.color.as_deref(), Some("DarkRed"));
        assert_eq!(cli.line_color.as_deref(), Some("#1e90ff"));
        assert_eq!(cli.bg, "white");
    }

    #[test]
    fn unknown_colors_are_rejected_when_parsing() {
        let err = parse(&["a.csv", "--marker-color", "notacolor"])
            .unwrap_err()
            .to_string();
        assert!(err.contains("unknown color 'notacolor'"), "{err}");
        assert!(parse(&["a.csv", "--bg", "#12345"]).is_err());
    }

    #[test]
    fn color_names_are_completed_but_not_listed_in_help() {
        let mut script = Vec::new();
        clap_complete::generate(Shell::Zsh, &mut Cli::command(), "termplt", &mut script);
        let script = String::from_utf8(script).unwrap();
        assert!(script.contains("dark-red"), "zsh script lacks color names");
        let help = Cli::command().render_long_help().to_string();
        assert!(!help.contains("dark-red"), "help lists every color");
    }

    #[test]
    fn unknown_flags_get_suggestions() {
        let err = parse(&["--xlimit", "0,1"]).unwrap_err().to_string();
        assert!(err.contains("--xlim"), "{err}");
    }
}
