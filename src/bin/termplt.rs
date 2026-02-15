use std::error::Error;
use std::fs;
use std::path::Path;

use rgb::RGB8;
use termplt::{
    get_window_size,
    kitty_graphics::ctrl_seq::{PixelFormat, Transmission},
    plotting::{
        axes::{Axes, AxesPositioning},
        canvas::{BufferType, TerminalCanvas},
        colors,
        graph::Graph,
        grid_lines::GridLines,
        line::LineStyle,
        marker::MarkerStyle,
        point::Point,
        series::Series,
        text::TextStyle,
    },
    terminal_commands::images::Image,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

// ---------------------------------------------------------------------------
// Default palette for automatic color/marker cycling
// ---------------------------------------------------------------------------

struct PaletteEntry {
    color: RGB8,
    marker_fn: fn(u32, RGB8) -> MarkerStyle,
}

const fn palette_entry(color: RGB8, marker_fn: fn(u32, RGB8) -> MarkerStyle) -> PaletteEntry {
    PaletteEntry { color, marker_fn }
}

fn filled_circle(size: u32, color: RGB8) -> MarkerStyle {
    MarkerStyle::FilledCircle { size, color }
}
fn hollow_circle(size: u32, color: RGB8) -> MarkerStyle {
    MarkerStyle::HollowCircle { size, color }
}
fn filled_square(size: u32, color: RGB8) -> MarkerStyle {
    MarkerStyle::FilledSquare { size, color }
}
fn hollow_square(size: u32, color: RGB8) -> MarkerStyle {
    MarkerStyle::HollowSquare { size, color }
}

const DEFAULT_PALETTE: &[PaletteEntry] = &[
    palette_entry(colors::BLUE, filled_circle),
    palette_entry(colors::RED, hollow_circle),
    palette_entry(colors::LIME, filled_square),
    palette_entry(colors::ORANGE, hollow_square),
    palette_entry(colors::CYAN, filled_circle),
    palette_entry(colors::MAGENTA, hollow_circle),
];

const DEFAULT_MARKER_SIZE: u32 = 2;
const DEFAULT_LINE_THICKNESS: u32 = 0;

// ---------------------------------------------------------------------------
// Series specification (parsed from CLI args)
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum DataSource {
    Inline(String),
    File(String),
}

#[derive(Debug)]
struct SeriesSpec {
    data_source: DataSource,
    marker_style: Option<String>,
    marker_color: Option<String>,
    marker_size: Option<u32>,
    line_style: Option<String>,
    line_color: Option<String>,
    line_thickness: Option<u32>,
}

impl SeriesSpec {
    fn new(data_source: DataSource) -> Self {
        SeriesSpec {
            data_source,
            marker_style: None,
            marker_color: None,
            marker_size: None,
            line_style: None,
            line_color: None,
            line_thickness: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Help
// ---------------------------------------------------------------------------

fn print_help(topic: Option<&str>) {
    match topic {
        Some("colors") => {
            println!("Available colors:\n");
            for (name, _) in colors::all_names() {
                println!("  {name}");
            }
        }
        Some("markers") => {
            println!("Available marker styles:\n");
            println!("  FilledCircle   (default)");
            println!("  HollowCircle");
            println!("  FilledSquare");
            println!("  HollowSquare");
            println!("  None           (line only, no markers)");
        }
        _ => {
            println!(
                "\
Usage: termplt [OPTIONS]

Render 2D plots in a Kitty-compatible terminal.

DATA (at least one required, repeat for multiple series):
  --data \"(x,y),(x,y),...\"   Inline data points
  --data_file <path>         Read x,y data from a file

STYLE (applies to the preceding --data or --data_file):
  --marker_style <style>     FilledCircle, HollowCircle, FilledSquare, HollowSquare, None
  --marker_color <color>     Named color (e.g. Blue, DARK_RED, lime)
  --marker_size <pixels>     Marker radius in pixels (default: {DEFAULT_MARKER_SIZE})
  --line_style <style>       Solid (default) or None (scatter plot, no connecting lines)
  --line_color <color>       Named color for connecting line
  --line_thickness <pixels>  Line thickness in pixels (default: {DEFAULT_LINE_THICKNESS})

OTHER:
  --verbose, -v              Print debug info (terminal size, canvas, buffer, etc.)
  --help, -h                 Show this help message
  --help colors              List all available color names
  --help markers             List all available marker styles

Examples:
  termplt --data \"(1,1),(2,4),(3,9)\"
  termplt --data_file data.csv --marker_color Red --line_color Red
  termplt --data_file a.txt --line_style None  (scatter plot, no lines)
  termplt --data_file a.txt --data_file b.txt"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Argument parsing
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct CliArgs {
    specs: Vec<SeriesSpec>,
    verbose: bool,
}

fn parse_args(args: Vec<String>) -> Result<CliArgs> {
    let mut specs: Vec<SeriesSpec> = Vec::new();
    let mut current: Option<SeriesSpec> = None;
    let mut verbose = false;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--verbose" | "-v" => {
                verbose = true;
            }
            "--help" | "-h" => {
                let topic = args.get(i + 1).map(|s| s.as_str());
                print_help(topic);
                std::process::exit(0);
            }
            "--data" => {
                if let Some(spec) = current.take() {
                    specs.push(spec);
                }
                i += 1;
                let val = args
                    .get(i)
                    .ok_or("--data requires a value, e.g. --data \"(1,2),(3,4)\"")?;
                current = Some(SeriesSpec::new(DataSource::Inline(val.clone())));
            }
            "--data_file" => {
                if let Some(spec) = current.take() {
                    specs.push(spec);
                }
                i += 1;
                let val = args
                    .get(i)
                    .ok_or("--data_file requires a file path")?;
                current = Some(SeriesSpec::new(DataSource::File(val.clone())));
            }
            "--marker_style" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or("--marker_style requires a value")?;
                let spec = current
                    .as_mut()
                    .ok_or("--marker_style must appear after --data or --data_file")?;
                spec.marker_style = Some(val.clone());
            }
            "--marker_color" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or("--marker_color requires a value")?;
                let spec = current
                    .as_mut()
                    .ok_or("--marker_color must appear after --data or --data_file")?;
                spec.marker_color = Some(val.clone());
            }
            "--marker_size" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or("--marker_size requires a value")?;
                let spec = current
                    .as_mut()
                    .ok_or("--marker_size must appear after --data or --data_file")?;
                spec.marker_size = Some(val.parse::<u32>().map_err(|_| {
                    format!("--marker_size value '{}' is not a valid positive integer", val)
                })?);
            }
            "--line_style" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or("--line_style requires a value")?;
                let spec = current
                    .as_mut()
                    .ok_or("--line_style must appear after --data or --data_file")?;
                spec.line_style = Some(val.clone());
            }
            "--line_color" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or("--line_color requires a value")?;
                let spec = current
                    .as_mut()
                    .ok_or("--line_color must appear after --data or --data_file")?;
                spec.line_color = Some(val.clone());
            }
            "--line_thickness" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or("--line_thickness requires a value")?;
                let spec = current
                    .as_mut()
                    .ok_or("--line_thickness must appear after --data or --data_file")?;
                spec.line_thickness = Some(val.parse::<u32>().map_err(|_| {
                    format!(
                        "--line_thickness value '{}' is not a valid positive integer",
                        val
                    )
                })?);
            }
            other => {
                return Err(format!(
                    "Unknown argument '{}'. Run 'termplt --help' for usage.",
                    other
                )
                .into());
            }
        }
        i += 1;
    }

    if let Some(spec) = current.take() {
        specs.push(spec);
    }

    if specs.is_empty() {
        return Err("No data provided. Use --data or --data_file to supply data points.\n\
                     Run 'termplt --help' for usage."
            .into());
    }

    Ok(CliArgs { specs, verbose })
}

// ---------------------------------------------------------------------------
// Data parsing
// ---------------------------------------------------------------------------

fn parse_inline_data(s: &str) -> Result<Vec<Point<f64>>> {
    let s = s.trim();
    if s.is_empty() {
        return Err("Inline data string is empty".into());
    }

    let mut points = Vec::new();
    // Strip leading/trailing parens from the whole string, then split on ),(
    let s = s.strip_prefix('(').unwrap_or(s);
    let s = s.strip_suffix(')').unwrap_or(s);

    for pair in s.split("),(") {
        let pair = pair.trim();
        let parts: Vec<&str> = pair.split(',').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid point '({})'. Expected format: (x,y)",
                pair
            )
            .into());
        }
        let x: f64 = parts[0].trim().parse().map_err(|_| {
            format!("Cannot parse x value '{}' as a number", parts[0].trim())
        })?;
        let y: f64 = parts[1].trim().parse().map_err(|_| {
            format!("Cannot parse y value '{}' as a number", parts[1].trim())
        })?;
        points.push(Point::new(x, y));
    }

    if points.is_empty() {
        return Err("No valid data points found in inline data".into());
    }

    Ok(points)
}

fn is_header_line(line: &str) -> bool {
    // A line is a header if the first non-whitespace, non-comment token cannot be parsed as f64
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return false; // skip lines, not headers
    }
    // Try to parse the first token as a number
    let first_token = trimmed
        .split(|c: char| c == ',' || c.is_whitespace())
        .next()
        .unwrap_or("");
    first_token.parse::<f64>().is_err()
}

fn parse_data_file(path: &Path) -> Result<Vec<Point<f64>>> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read file '{}': {}", path.display(), e))?;

    let mut points = Vec::new();
    let mut lines = content.lines().peekable();

    // Auto-detect and skip header
    if lines.peek().is_some_and(|line| is_header_line(line)) {
        lines.next();
    }

    for (line_num, line) in lines.enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Split on comma or whitespace
        let tokens: Vec<&str> = trimmed
            .split([',', '\t'])
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .collect();

        // If comma/tab split didn't work (single token), try whitespace
        let tokens = if tokens.len() == 1 {
            trimmed.split_whitespace().collect::<Vec<&str>>()
        } else {
            tokens
        };

        if tokens.len() < 2 {
            return Err(format!(
                "{}:{}: expected at least 2 values (x, y), got {}",
                path.display(),
                line_num + 1,
                tokens.len()
            )
            .into());
        }

        let x: f64 = tokens[0].parse().map_err(|_| {
            format!(
                "{}:{}: cannot parse x value '{}' as a number",
                path.display(),
                line_num + 1,
                tokens[0]
            )
        })?;
        let y: f64 = tokens[1].parse().map_err(|_| {
            format!(
                "{}:{}: cannot parse y value '{}' as a number",
                path.display(),
                line_num + 1,
                tokens[1]
            )
        })?;

        points.push(Point::new(x, y));
    }

    if points.is_empty() {
        return Err(format!("No data points found in '{}'", path.display()).into());
    }

    Ok(points)
}

// ---------------------------------------------------------------------------
// Color and marker style resolution
// ---------------------------------------------------------------------------

fn resolve_color(name: &str) -> Result<RGB8> {
    colors::from_name(name).ok_or_else(|| {
        let preview: Vec<&str> = colors::all_names()
            .iter()
            .take(8)
            .map(|(n, _)| *n)
            .collect();
        format!(
            "Unknown color '{}'. Valid colors: {}, ... (run 'termplt --help colors' for full list)",
            name,
            preview.join(", ")
        )
        .into()
    })
}

fn resolve_marker_style(name: &str, size: u32, color: RGB8) -> Result<Option<MarkerStyle>> {
    match name.to_ascii_lowercase().as_str() {
        "filledcircle" => Ok(Some(MarkerStyle::FilledCircle { size, color })),
        "hollowcircle" => Ok(Some(MarkerStyle::HollowCircle { size, color })),
        "filledsquare" => Ok(Some(MarkerStyle::FilledSquare { size, color })),
        "hollowsquare" => Ok(Some(MarkerStyle::HollowSquare { size, color })),
        "none" => Ok(None),
        _ => Err(format!(
            "Unknown marker style '{}'. Valid styles: FilledCircle, HollowCircle, \
             FilledSquare, HollowSquare, None",
            name
        )
        .into()),
    }
}

// ---------------------------------------------------------------------------
// Series building
// ---------------------------------------------------------------------------

fn build_series(spec: SeriesSpec, index: usize) -> Result<Series<f64>> {
    let points = match &spec.data_source {
        DataSource::Inline(s) => parse_inline_data(s)?,
        DataSource::File(p) => parse_data_file(Path::new(p))?,
    };

    let palette = &DEFAULT_PALETTE[index % DEFAULT_PALETTE.len()];
    let marker_size = spec.marker_size.unwrap_or(DEFAULT_MARKER_SIZE);
    let line_thickness = spec.line_thickness.unwrap_or(DEFAULT_LINE_THICKNESS);

    // Resolve colors — if only one is set, the other matches it
    let default_color = palette.color;
    let marker_color = spec
        .marker_color
        .as_deref()
        .map(resolve_color)
        .transpose()?;
    let line_color = spec
        .line_color
        .as_deref()
        .map(resolve_color)
        .transpose()?;

    let effective_marker_color = marker_color.or(line_color).unwrap_or(default_color);
    let effective_line_color = line_color.or(marker_color).unwrap_or(default_color);

    // Build marker style
    let marker_style = if let Some(style_name) = &spec.marker_style {
        resolve_marker_style(style_name, marker_size, effective_marker_color)?
    } else {
        Some((palette.marker_fn)(marker_size, effective_marker_color))
    };

    let mut series = Series::new(&points);

    if let Some(ms) = marker_style {
        series = series.with_marker_style(ms);
    } else {
        // "None" marker — use zero-size invisible marker
        series = series.with_marker_style(MarkerStyle::FilledSquare {
            size: 0,
            color: RGB8::new(0, 0, 0),
        });
    }

    // Resolve line style — "None" means no connecting lines (scatter plot)
    let wants_line = match spec.line_style.as_deref() {
        Some(s) if s.eq_ignore_ascii_case("none") => false,
        Some(s) if s.eq_ignore_ascii_case("solid") => true,
        Some(s) => {
            return Err(format!(
                "Unknown line style '{}'. Valid styles: Solid, None",
                s
            )
            .into())
        }
        None => true, // default: draw lines
    };

    if wants_line {
        series = series.with_line_style(LineStyle::Solid {
            color: effective_line_color,
            thickness: line_thickness,
        });
    }

    Ok(series)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        print_help(None);
        return Ok(());
    }

    let cli = parse_args(args)?;
    let verbose = cli.verbose;

    // Build all series
    let mut graph = Graph::new();
    for (i, spec) in cli.specs.into_iter().enumerate() {
        if verbose {
            eprintln!("[verbose] series {}: {:?}", i, spec);
        }
        let series = build_series(spec, i)?;
        if verbose {
            eprintln!(
                "[verbose] series {}: {} points, marker={:?}, line={:?}",
                i,
                series.data().len(),
                series.marker_style(),
                series.line_style()
            );
        }
        graph = graph.with_series(series);
    }

    // Add axes and grid lines
    let axes_thickness: u32 = 1;
    graph = graph
        .with_axes(Axes::new(
            AxesPositioning::XY(LineStyle::Solid {
                color: colors::WHITE,
                thickness: axes_thickness,
            }),
            TextStyle::with_color(colors::WHITE),
        ))
        .with_grid_lines(GridLines::XY(LineStyle::Solid {
            color: colors::GRAY,
            thickness: 0,
        }));

    // Determine canvas size from terminal window
    let win = get_window_size()?;
    if verbose {
        eprintln!(
            "[verbose] terminal: {}x{} cells, {}x{} pixels ({} px/col, {} px/row)",
            win.cols, win.rows, win.x_pix, win.y_pix, win.pix_per_col, win.pix_per_row
        );
    }

    let size = std::cmp::min(win.x_pix, win.y_pix) / 2;
    let size = std::cmp::max(size, 200); // minimum 200px
    let width = size;
    let height = size;
    let buffer = std::cmp::max(size / 10, 20);

    if verbose {
        eprintln!("[verbose] canvas: {}x{} pixels", width, height);
        eprintln!("[verbose] buffer: {} pixels (uniform)", buffer);

        let largest_marker = graph
            .data()
            .iter()
            .map(|s| s.marker_style().size())
            .max()
            .unwrap_or(0);
        let axes_bound = 2 * axes_thickness;
        let inset = u32::max(largest_marker, axes_bound);
        let drawable_w = (width - 1).saturating_sub(2 * (buffer + inset));
        let drawable_h = (height - 1).saturating_sub(2 * (buffer + inset));
        eprintln!(
            "[verbose] largest marker: {}, axes bound: {}, effective inset: {}",
            largest_marker, axes_bound, inset
        );
        eprintln!(
            "[verbose] estimated drawable area: ~{}x{} pixels",
            drawable_w, drawable_h
        );
        if drawable_w == 0 || drawable_h == 0 {
            eprintln!(
                "[verbose] WARNING: drawable area is zero! Canvas {}x{} is too small \
                 for buffer ({}) + inset ({}). Consider a larger terminal window.",
                width, height, buffer, inset
            );
        }
    }

    let bytes = TerminalCanvas::new(width, height, colors::BLACK)
        .with_buffer(BufferType::Uniform(buffer))
        .with_graph(graph)
        .draw()?
        .get_bytes();

    Image::new(
        PixelFormat::Rgb { width, height },
        Transmission::Direct(bytes),
    )?
    .display()?;

    // Print a newline so the shell prompt appears below the image
    println!();

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- parse_args tests --

    #[test]
    fn parse_args_single_inline_data() {
        let args = vec!["--data".into(), "(1,2),(3,4)".into()];
        let cli = parse_args(args).unwrap();
        assert_eq!(cli.specs.len(), 1);
        assert!(matches!(cli.specs[0].data_source, DataSource::Inline(_)));
        assert!(!cli.verbose);
    }

    #[test]
    fn parse_args_single_file_data() {
        let args = vec!["--data_file".into(), "test.csv".into()];
        let cli = parse_args(args).unwrap();
        assert_eq!(cli.specs.len(), 1);
        assert!(matches!(cli.specs[0].data_source, DataSource::File(_)));
    }

    #[test]
    fn parse_args_multiple_series() {
        let args = vec![
            "--data".into(),
            "(1,2)".into(),
            "--data_file".into(),
            "b.txt".into(),
        ];
        let cli = parse_args(args).unwrap();
        assert_eq!(cli.specs.len(), 2);
    }

    #[test]
    fn parse_args_with_style_flags() {
        let args = vec![
            "--data".into(),
            "(1,2),(3,4)".into(),
            "--marker_style".into(),
            "HollowCircle".into(),
            "--marker_color".into(),
            "Red".into(),
            "--marker_size".into(),
            "5".into(),
            "--line_color".into(),
            "Blue".into(),
            "--line_thickness".into(),
            "2".into(),
        ];
        let cli = parse_args(args).unwrap();
        let specs = &cli.specs;
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].marker_style.as_deref(), Some("HollowCircle"));
        assert_eq!(specs[0].marker_color.as_deref(), Some("Red"));
        assert_eq!(specs[0].marker_size, Some(5));
        assert_eq!(specs[0].line_color.as_deref(), Some("Blue"));
        assert_eq!(specs[0].line_thickness, Some(2));
    }

    #[test]
    fn parse_args_style_before_data_errors() {
        let args = vec!["--marker_color".into(), "Red".into()];
        let result = parse_args(args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must appear after"));
    }

    #[test]
    fn parse_args_no_data_errors() {
        let args: Vec<String> = vec![];
        let result = parse_args(args);
        assert!(result.is_err());
    }

    #[test]
    fn parse_args_unknown_flag_errors() {
        let args = vec!["--bogus".into()];
        let result = parse_args(args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown argument"));
    }

    #[test]
    fn parse_args_missing_data_value_errors() {
        let args = vec!["--data".into()];
        let result = parse_args(args);
        assert!(result.is_err());
    }

    // -- parse_inline_data tests --

    #[test]
    fn parse_inline_data_basic() {
        let points = parse_inline_data("(1,2),(3,4),(5,6)").unwrap();
        assert_eq!(points.len(), 3);
        assert_eq!(points[0], Point::new(1.0, 2.0));
        assert_eq!(points[1], Point::new(3.0, 4.0));
        assert_eq!(points[2], Point::new(5.0, 6.0));
    }

    #[test]
    fn parse_inline_data_single_point() {
        let points = parse_inline_data("(1.5,2.5)").unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0], Point::new(1.5, 2.5));
    }

    #[test]
    fn parse_inline_data_negative_values() {
        let points = parse_inline_data("(-1,-2),(3.5,-4.5)").unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0], Point::new(-1.0, -2.0));
        assert_eq!(points[1], Point::new(3.5, -4.5));
    }

    #[test]
    fn parse_inline_data_with_spaces() {
        let points = parse_inline_data("( 1 , 2 ),( 3 , 4 )").unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0], Point::new(1.0, 2.0));
    }

    #[test]
    fn parse_inline_data_empty_errors() {
        assert!(parse_inline_data("").is_err());
    }

    #[test]
    fn parse_inline_data_malformed_errors() {
        assert!(parse_inline_data("(1,2,3)").is_err());
    }

    // -- parse_data_file tests --

    #[test]
    fn parse_data_file_csv_no_header() {
        let dir = std::env::temp_dir();
        let path = dir.join("termplt_test_no_header.csv");
        fs::write(&path, "1,2\n3,4\n5,6\n").unwrap();
        let points = parse_data_file(&path).unwrap();
        assert_eq!(points.len(), 3);
        assert_eq!(points[0], Point::new(1.0, 2.0));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn parse_data_file_csv_with_header() {
        let dir = std::env::temp_dir();
        let path = dir.join("termplt_test_with_header.csv");
        fs::write(&path, "x,y\n1,2\n3,4\n").unwrap();
        let points = parse_data_file(&path).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0], Point::new(1.0, 2.0));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn parse_data_file_with_comments_and_blanks() {
        let dir = std::env::temp_dir();
        let path = dir.join("termplt_test_comments.csv");
        fs::write(&path, "# comment\n1,2\n\n# another comment\n3,4\n").unwrap();
        let points = parse_data_file(&path).unwrap();
        assert_eq!(points.len(), 2);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn parse_data_file_whitespace_delimited() {
        let dir = std::env::temp_dir();
        let path = dir.join("termplt_test_whitespace.txt");
        fs::write(&path, "1.0 2.0\n3.0 4.0\n").unwrap();
        let points = parse_data_file(&path).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0], Point::new(1.0, 2.0));
        fs::remove_file(&path).ok();
    }

    #[test]
    fn parse_data_file_tab_delimited() {
        let dir = std::env::temp_dir();
        let path = dir.join("termplt_test_tab.tsv");
        fs::write(&path, "1.0\t2.0\n3.0\t4.0\n").unwrap();
        let points = parse_data_file(&path).unwrap();
        assert_eq!(points.len(), 2);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn parse_data_file_missing_file_errors() {
        let result = parse_data_file(Path::new("/nonexistent/path.csv"));
        assert!(result.is_err());
    }

    #[test]
    fn parse_data_file_empty_file_errors() {
        let dir = std::env::temp_dir();
        let path = dir.join("termplt_test_empty.csv");
        fs::write(&path, "").unwrap();
        let result = parse_data_file(&path);
        assert!(result.is_err());
        fs::remove_file(&path).ok();
    }

    // -- resolve_marker_style tests --

    #[test]
    fn resolve_marker_style_valid() {
        let color = colors::RED;
        assert!(resolve_marker_style("FilledCircle", 2, color)
            .unwrap()
            .is_some());
        assert!(resolve_marker_style("hollowcircle", 2, color)
            .unwrap()
            .is_some());
        assert!(resolve_marker_style("FILLEDSQUARE", 2, color)
            .unwrap()
            .is_some());
        assert!(resolve_marker_style("None", 2, color)
            .unwrap()
            .is_none());
    }

    #[test]
    fn resolve_marker_style_invalid_errors() {
        assert!(resolve_marker_style("Triangle", 2, colors::RED).is_err());
    }

    // -- resolve_color tests --

    #[test]
    fn resolve_color_valid() {
        assert_eq!(resolve_color("Blue").unwrap(), colors::BLUE);
        assert_eq!(resolve_color("red").unwrap(), colors::RED);
    }

    #[test]
    fn resolve_color_invalid_errors() {
        let result = resolve_color("Gren");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown color"));
    }

    // -- build_series tests --

    #[test]
    fn build_series_with_defaults() {
        let spec = SeriesSpec::new(DataSource::Inline("(1,2),(3,4)".into()));
        let series = build_series(spec, 0).unwrap();
        assert_eq!(series.data().len(), 2);
    }

    #[test]
    fn build_series_color_matching() {
        // When only marker_color is set, line should match
        let mut spec = SeriesSpec::new(DataSource::Inline("(1,2),(3,4)".into()));
        spec.marker_color = Some("Lime".into());
        let series = build_series(spec, 0).unwrap();
        // Verify the line style exists and has the right color
        match series.line_style() {
            Some(LineStyle::Solid { color, .. }) => assert_eq!(*color, colors::LIME),
            _ => panic!("Expected solid line style"),
        }
    }

    #[test]
    fn build_series_line_none_produces_scatter() {
        let mut spec = SeriesSpec::new(DataSource::Inline("(1,2),(3,4)".into()));
        spec.line_style = Some("None".into());
        let series = build_series(spec, 0).unwrap();
        assert!(series.line_style().is_none());
    }

    #[test]
    fn build_series_line_solid_explicit() {
        let mut spec = SeriesSpec::new(DataSource::Inline("(1,2),(3,4)".into()));
        spec.line_style = Some("Solid".into());
        let series = build_series(spec, 0).unwrap();
        assert!(series.line_style().is_some());
    }

    #[test]
    fn build_series_line_invalid_errors() {
        let mut spec = SeriesSpec::new(DataSource::Inline("(1,2),(3,4)".into()));
        spec.line_style = Some("Dotted".into());
        assert!(build_series(spec, 0).is_err());
    }

    #[test]
    fn parse_args_line_style_flag() {
        let args = vec![
            "--data".into(),
            "(1,2)".into(),
            "--line_style".into(),
            "None".into(),
        ];
        let cli = parse_args(args).unwrap();
        assert_eq!(cli.specs[0].line_style.as_deref(), Some("None"));
    }

    #[test]
    fn parse_args_verbose_flag() {
        let args = vec!["-v".into(), "--data".into(), "(1,2)".into()];
        let cli = parse_args(args).unwrap();
        assert!(cli.verbose);
        assert_eq!(cli.specs.len(), 1);
    }

    #[test]
    fn parse_args_verbose_flag_long() {
        let args = vec!["--data".into(), "(1,2)".into(), "--verbose".into()];
        let cli = parse_args(args).unwrap();
        assert!(cli.verbose);
    }

    #[test]
    fn build_series_marker_none() {
        let mut spec = SeriesSpec::new(DataSource::Inline("(1,2),(3,4)".into()));
        spec.marker_style = Some("None".into());
        let series = build_series(spec, 0).unwrap();
        // Should have zero-size marker
        assert_eq!(series.marker_style().size(), 0);
    }
}
