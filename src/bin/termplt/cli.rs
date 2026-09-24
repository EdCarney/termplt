//! Command-line arguments.

use clap::Parser;
use std::path::PathBuf;

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
    #[arg(value_name = "FILE")]
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
    #[arg(short, long, value_name = "COLOR", help_heading = "Style")]
    pub color: Option<String>,

    /// Marker style: filled-circle, hollow-circle, filled-square, hollow-square or none
    #[arg(
        long,
        value_name = "STYLE",
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
        alias = "marker_color",
        help_heading = "Style"
    )]
    pub marker_color: Option<String>,

    /// Line style: solid, dashed or none (scatter plot) [default: solid]
    #[arg(
        long,
        value_name = "STYLE",
        alias = "line-style",
        alias = "line_style",
        help_heading = "Style"
    )]
    pub line: Option<String>,

    /// Line color (overrides --color)
    #[arg(
        long,
        value_name = "COLOR",
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
        default_value = "black",
        help_heading = "Plot"
    )]
    pub bg: String,

    /// Hide grid lines
    #[arg(long, help_heading = "Plot")]
    pub no_grid: bool,

    /// Write the plot to an image file (e.g. plot.png) instead of displaying it; no terminal
    /// is needed
    #[arg(short, long, value_name = "FILE", help_heading = "Plot")]
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
    #[arg(long, value_name = "FILE", alias = "data_file", hide = true)]
    pub data_file: Vec<String>,
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
    fn unknown_flags_get_suggestions() {
        let err = parse(&["--xlimit", "0,1"]).unwrap_err().to_string();
        assert!(err.contains("--xlim"), "{err}");
    }
}
