mod cli;
mod data;
mod series;

use clap::{CommandFactory, Parser};
use cli::Cli;
use data::{Column, Table};
use rgb::RGB8;
use series::{SeriesSpec, Source, Style};
use std::{
    error::Error,
    fs,
    io::{self, IsTerminal, Read},
    path::Path,
};
use termplt::{
    WindowSize, get_window_size,
    plotting::{
        axes::{Axes, AxesPositioning},
        canvas::{BufferType, TerminalCanvas},
        colors,
        graph::Graph,
        grid_lines::GridLines,
        line::LineStyle,
        text::TextStyle,
    },
    terminal_commands::{
        images::Image,
        kitty_cmds::{self, GraphicsSupportError, Passthrough},
        responses::TerminalCommandError,
    },
};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Image size used with --output when no size is given.
const DEFAULT_OUTPUT_SIZE: (u32, u32) = (800, 600);
/// Smallest size used when fitting the image to the terminal.
const MIN_SIZE: (u32, u32) = (200, 150);
/// Cell size assumed when the terminal's pixel size is unknown (a common size for 12-14 pt
/// fonts).
const ASSUMED_CELL_SIZE: (u32, u32) = (9, 18);

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    if let Some(shell) = cli.completions {
        clap_complete::generate(shell, &mut Cli::command(), "termplt", &mut io::stdout());
        return Ok(());
    }
    if cli.list_colors {
        println!("Colors (names ignore case and separators, e.g. DarkRed, dark-red):\n");
        for (name, _) in colors::all_names() {
            println!("  {}", cli::color_display_name(name));
        }
        println!("\nAny #RRGGBB or #RGB hex color is also accepted.");
        return Ok(());
    }
    if cli.list_markers {
        println!("Marker styles:\n");
        for (name, description) in series::MARKER_NAMES {
            println!("  {name:<15} {description}");
        }
        return Ok(());
    }

    let stdin_is_terminal = io::stdin().is_terminal();
    let specs = collect_specs(&cli, !stdin_is_terminal)?;
    if specs.is_empty() {
        Cli::command().print_help()?;
        return Ok(());
    }

    let defaults = Style {
        color: cli.color.clone(),
        marker: cli.marker.clone(),
        marker_size: cli.marker_size,
        marker_color: cli.marker_color.clone(),
        line: cli.line.clone(),
        line_color: cli.line_color.clone(),
        line_thickness: cli.line_thickness,
    };

    let mut stdin_cache = None;
    let mut graph = Graph::new();
    for (index, spec) in specs.iter().enumerate() {
        let points = load_points(spec, &mut stdin_cache)?;
        let series = series::build_series(&points, &spec.style.or(&defaults), index)?;
        if cli.verbose {
            eprintln!(
                "[verbose] series {index}: {} points from {}, marker={:?}, line={:?}",
                points.len(),
                spec.source.describe(),
                series.marker_style(),
                series.line_style()
            );
        }
        graph = graph.with_series(series);
    }
    if let Some((min, max)) = cli.xlim {
        graph = graph.with_x_limits(min, max);
    }
    if let Some((min, max)) = cli.ylim {
        graph = graph.with_y_limits(min, max);
    }

    let background = series::parse_color(&cli.bg)?;
    let (foreground, grid_color) = if luminance(background) > 127.5 {
        (colors::BLACK, colors::LIGHT_GRAY)
    } else {
        (colors::WHITE, colors::GRAY)
    };
    graph = graph.with_axes(Axes::new(
        AxesPositioning::XY(LineStyle::Solid {
            color: foreground,
            thickness: 1,
        }),
        TextStyle::with_color(foreground),
    ));
    if !cli.no_grid {
        graph = graph.with_grid_lines(GridLines::XY(LineStyle::Solid {
            color: grid_color,
            thickness: 0,
        }));
    }

    // an image file needs no terminal; displaying one needs a terminal to size it and draw on
    let passthrough = Passthrough::detect();
    let window = match &cli.output {
        Some(_) => None,
        None => Some(prepare_terminal(passthrough, cli.verbose)?),
    };

    let (width, height) = canvas_size(cli.width, cli.height, window.as_ref());
    // tick labels are laid out inside the canvas automatically; the buffer is just breathing
    // room around the edges
    let buffer = (width.min(height) / 40).max(8);
    let canvas = TerminalCanvas::new(width, height, background)
        .with_buffer(BufferType::Uniform(buffer))
        .with_graph(graph);

    if cli.verbose {
        eprintln!("[verbose] canvas: {width}x{height} pixels, buffer: {buffer} pixels");
        match canvas.get_drawable_limits() {
            Ok(plot) => {
                let (w, h) = plot.span();
                eprintln!(
                    "[verbose] plot area: {w}x{h} pixels at ({}, {})",
                    plot.min().x,
                    plot.min().y
                );
            }
            Err(e) => eprintln!("[verbose] plot area unavailable: {e}"),
        }
    }

    let bytes = canvas.draw()?.get_bytes();

    match &cli.output {
        Some(path) => {
            image::save_buffer(path, &bytes, width, height, image::ColorType::Rgb8)
                .map_err(|e| format!("cannot write '{}': {e}", path.display()))?;
            if cli.verbose {
                eprintln!("[verbose] wrote {}", path.display());
            }
        }
        None => {
            let image = Image::png_from_rgb(&bytes, width, height)?;
            if cli.verbose {
                eprintln!(
                    "[verbose] sending {} bytes of PNG data ({} bytes uncompressed)",
                    image.payload_len(),
                    bytes.len()
                );
            }
            match (passthrough, &window) {
                (Passthrough::Tmux, Some(window)) => {
                    // tmux doesn't know the image is there, so it wouldn't account for the
                    // terminal moving the cursor below it; move the cursor ourselves instead
                    image.display_without_moving_cursor()?;
                    let rows = height.div_ceil(window.pix_per_row.max(1));
                    print!("{}", "\n".repeat(rows as usize));
                }
                _ => {
                    image.display()?;
                    // Print a newline so the shell prompt appears below the image
                    println!();
                }
            }
        }
    }

    Ok(())
}

/// Checks that stdout is a terminal that can show images and returns its size.
fn prepare_terminal(passthrough: Passthrough, verbose: bool) -> Result<WindowSize> {
    if !io::stdout().is_terminal() {
        return Err(
            "stdout is not a terminal. termplt draws plots using the Kitty graphics \
                    protocol and must write to a terminal that supports it (e.g. Kitty, \
                    WezTerm, Ghostty); use --output plot.png to write an image file instead."
                .into(),
        );
    }

    // inside tmux the outer terminal's reply never reaches us, so there is nothing to check
    if passthrough == Passthrough::None {
        match kitty_cmds::query_support() {
            Ok(()) => {
                if verbose {
                    eprintln!("[verbose] terminal supports the Kitty graphics protocol");
                }
            }
            Err(e) if e.is::<GraphicsSupportError>() => {
                return Err(
                    format!("{e}. Use --output plot.png to write an image file instead.").into(),
                );
            }
            // a terminal that answers nothing at all may still draw images; try anyway
            Err(e) => match e.downcast_ref::<TerminalCommandError>() {
                Some(TerminalCommandError::Timeout(_)) => eprintln!(
                    "warning: the terminal did not answer a graphics support query; \
                     drawing anyway"
                ),
                _ => eprintln!("warning: could not check for graphics support: {e}"),
            },
        }
    } else {
        if tmux_passthrough_disabled() {
            return Err("tmux is blocking the image: enable passthrough with \
                        `tmux set -g allow-passthrough on` (add `set -g allow-passthrough on` to \
                        ~/.tmux.conf to keep it), or use --output plot.png to write an image file \
                        instead."
                .into());
        }
        if verbose {
            eprintln!("[verbose] inside tmux: sending images through tmux passthrough");
        }
    }

    let window = match get_window_size() {
        Ok(window) => window,
        Err(e) => {
            let window = estimate_window_size().ok_or(e)?;
            eprintln!(
                "warning: the terminal did not report its size in pixels; assuming {}x{} pixel \
                 cells. Use --width/--height to set the plot size.",
                window.pix_per_col, window.pix_per_row
            );
            window
        }
    };
    if verbose {
        eprintln!(
            "[verbose] terminal: {}x{} cells, {}x{} pixels ({} px/col, {} px/row)",
            window.cols,
            window.rows,
            window.x_pix,
            window.y_pix,
            window.pix_per_col,
            window.pix_per_row
        );
    }
    Ok(window)
}

/// Whether tmux's `allow-passthrough` option is off for this pane (the default), in which case
/// tmux silently drops the image. Versions before 3.3 have no such option and always pass it on.
fn tmux_passthrough_disabled() -> bool {
    std::process::Command::new("tmux")
        .args(["show-options", "-pAv", "allow-passthrough"])
        .stderr(std::process::Stdio::null())
        .output()
        .is_ok_and(|out| {
            out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "off"
        })
}

/// A window size from the cell count alone, for terminals that don't report pixel sizes.
fn estimate_window_size() -> Option<WindowSize> {
    let (cols, rows) = crossterm::terminal::size().ok()?;
    let (cols, rows) = (u32::from(cols), u32::from(rows));
    if cols == 0 || rows == 0 {
        return None;
    }
    let (pix_per_col, pix_per_row) = ASSUMED_CELL_SIZE;
    Some(WindowSize {
        rows,
        cols,
        x_pix: cols * pix_per_col,
        y_pix: rows * pix_per_row,
        pix_per_col,
        pix_per_row,
    })
}

/// Collects the series to plot, in order: FILE arguments (one series per y column), --data,
/// then --series. Stdin is used when it is piped and no other data is given.
fn collect_specs(cli: &Cli, stdin_is_piped: bool) -> Result<Vec<SeriesSpec>> {
    let x = cli.x_col.as_deref().map(Column::parse).transpose()?;
    let y_cols = cli
        .y_col
        .iter()
        .map(|c| Column::parse(c))
        .collect::<Result<Vec<_>>>()?;

    let mut files: Vec<&String> = cli.files.iter().chain(&cli.data_file).collect();
    let stdin = "-".to_string();
    if files.is_empty() && cli.data.is_empty() && cli.series.is_empty() && stdin_is_piped {
        files.push(&stdin);
    }

    let mut specs = Vec::new();
    for file in files {
        // no --y-col means "the default column" (None), which also allows single-column files
        let ys: Vec<Option<Column>> = if y_cols.is_empty() {
            vec![None]
        } else {
            y_cols.iter().cloned().map(Some).collect()
        };
        for y in ys {
            let mut spec = SeriesSpec::new(Source::File(file.clone()));
            spec.x = x.clone();
            spec.y = y;
            specs.push(spec);
        }
    }
    for data in &cli.data {
        specs.push(SeriesSpec::new(Source::Inline(data.clone())));
    }
    for spec in &cli.series {
        let mut spec = series::parse_spec(spec)?;
        // file series default to the global column choices
        if matches!(spec.source, Source::File(_)) {
            spec.x = spec.x.or_else(|| x.clone());
            if spec.y.is_none() {
                spec.y = y_cols.first().cloned();
            }
        }
        specs.push(spec);
    }
    Ok(specs)
}

/// Reads and parses a series' points, skipping (with a warning) rows with missing values and
/// points with NaN or infinite coordinates. Stdin is read at most once and shared.
fn load_points(
    spec: &SeriesSpec,
    stdin_cache: &mut Option<String>,
) -> Result<Vec<termplt::plotting::point::Point<f64>>> {
    let source = spec.source.describe();
    let (points, skipped_missing) = match &spec.source {
        Source::Inline(s) => {
            let points = data::parse_inline(s).map_err(|e| format!("{source}: {e}"))?;
            (points, 0)
        }
        Source::File(path) => {
            let content = if path == "-" {
                if stdin_cache.is_none() {
                    let mut content = String::new();
                    io::stdin()
                        .read_to_string(&mut content)
                        .map_err(|e| format!("cannot read stdin: {e}"))?;
                    *stdin_cache = Some(content);
                }
                stdin_cache.clone().unwrap_or_default()
            } else {
                fs::read_to_string(Path::new(path))
                    .map_err(|e| format!("cannot read file '{path}': {e}"))?
            };
            let name = if path == "-" { "stdin" } else { path.as_str() };
            let parsed = Table::parse(&content).points(spec.x.as_ref(), spec.y.as_ref(), name)?;
            (parsed.points, parsed.skipped_missing)
        }
    };

    if skipped_missing > 0 {
        eprintln!("warning: skipped {skipped_missing} row(s) with missing values in {source}");
    }

    let total = points.len();
    let points: Vec<_> = points
        .into_iter()
        .filter(|p| p.x.is_finite() && p.y.is_finite())
        .collect();
    if points.len() < total {
        eprintln!(
            "warning: skipped {} point(s) with NaN or infinite values in {source}",
            total - points.len()
        );
    }
    if points.is_empty() {
        return Err(format!("no data points found in {source}").into());
    }
    Ok(points)
}

/// Chooses the image size: explicit sizes win; otherwise fit the terminal (full width, 60% of
/// the height, at most twice as wide as tall), or use a fixed size when writing a file.
fn canvas_size(width: Option<u32>, height: Option<u32>, window: Option<&WindowSize>) -> (u32, u32) {
    let (default_w, default_h) = match window {
        None => DEFAULT_OUTPUT_SIZE,
        Some(window) => {
            // leave one column free so the image does not wrap
            let available_w = window.x_pix.saturating_sub(window.pix_per_col);
            let h = (window.y_pix * 3 / 5).max(MIN_SIZE.1);
            let w = available_w.min(2 * h).max(MIN_SIZE.0);
            (w, h)
        }
    };
    (width.unwrap_or(default_w), height.unwrap_or(default_h))
}

fn luminance(color: RGB8) -> f64 {
    0.2126 * color.r as f64 + 0.7152 * color.g as f64 + 0.0722 * color.b as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli(args: &[&str]) -> Cli {
        Cli::try_parse_from(std::iter::once("termplt").chain(args.iter().copied())).unwrap()
    }

    fn window(x_pix: u32, y_pix: u32) -> WindowSize {
        WindowSize {
            rows: y_pix / 20,
            cols: x_pix / 10,
            x_pix,
            y_pix,
            pix_per_row: 20,
            pix_per_col: 10,
        }
    }

    #[test]
    fn specs_from_files_data_and_series_in_order() {
        let specs = collect_specs(
            &cli(&["a.csv", "-d", "(1,2)", "-s", "file=b.csv,color=red"]),
            false,
        )
        .unwrap();
        let sources: Vec<_> = specs.iter().map(|s| s.source.clone()).collect();
        assert_eq!(
            sources,
            [
                Source::File("a.csv".into()),
                Source::Inline("(1,2)".into()),
                Source::File("b.csv".into())
            ]
        );
    }

    #[test]
    fn several_y_columns_make_several_series() {
        let specs = collect_specs(&cli(&["a.csv", "-x", "time", "-y", "t,h"]), false).unwrap();
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].x, Some(Column::Name("time".into())));
        assert_eq!(specs[0].y, Some(Column::Name("t".into())));
        assert_eq!(specs[1].y, Some(Column::Name("h".into())));
    }

    #[test]
    fn series_files_inherit_global_columns() {
        let specs =
            collect_specs(&cli(&["-x", "2", "-y", "3", "-s", "file=a.csv"]), false).unwrap();
        assert_eq!(specs[0].x, Some(Column::Index(1)));
        assert_eq!(specs[0].y, Some(Column::Index(2)));
        let specs = collect_specs(&cli(&["-y", "3", "-s", "file=a.csv,y=4"]), false).unwrap();
        assert_eq!(specs[0].y, Some(Column::Index(3)));
    }

    #[test]
    fn piped_stdin_is_used_only_without_other_data() {
        let specs = collect_specs(&cli(&[]), true).unwrap();
        assert_eq!(specs[0].source, Source::File("-".into()));
        assert!(collect_specs(&cli(&[]), false).unwrap().is_empty());
        let specs = collect_specs(&cli(&["a.csv"]), true).unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].source, Source::File("a.csv".into()));
    }

    #[test]
    fn legacy_data_file_flag_is_a_file() {
        let specs = collect_specs(&cli(&["--data_file", "a.csv"]), false).unwrap();
        assert_eq!(specs[0].source, Source::File("a.csv".into()));
    }

    #[test]
    fn canvas_size_defaults() {
        assert_eq!(canvas_size(None, None, None), DEFAULT_OUTPUT_SIZE);
        assert_eq!(canvas_size(Some(300), None, None), (300, 600));
        // wide terminal: 60% of the height, width capped at twice the height
        assert_eq!(
            canvas_size(None, None, Some(&window(1600, 1000))),
            (1200, 600)
        );
        // narrow terminal: full width minus one column
        assert_eq!(
            canvas_size(None, None, Some(&window(500, 1000))),
            (490, 600)
        );
        // tiny terminal: minimum size
        assert_eq!(canvas_size(None, None, Some(&window(100, 100))), MIN_SIZE);
        assert_eq!(
            canvas_size(Some(640), Some(480), Some(&window(1600, 1000))),
            (640, 480)
        );
    }

    #[test]
    fn load_points_reports_missing_source() {
        let spec = SeriesSpec::new(Source::File("/definitely/not/here.csv".into()));
        let err = load_points(&spec, &mut None).unwrap_err().to_string();
        assert!(err.contains("cannot read file"), "{err}");
    }

    #[test]
    fn load_points_skips_non_finite_values() {
        let spec = SeriesSpec::new(Source::Inline("(1,nan),(2,2),(inf,3),(4,4)".into()));
        let points = load_points(&spec, &mut None).unwrap();
        assert_eq!(points.len(), 2);
        let spec = SeriesSpec::new(Source::Inline("(nan,1),(2,-inf)".into()));
        assert!(load_points(&spec, &mut None).is_err());
    }

    #[test]
    fn load_points_reuses_cached_stdin() {
        let spec = SeriesSpec::new(Source::File("-".into()));
        let mut cache = Some("x,y\n1,2\n3,4\n".to_string());
        assert_eq!(load_points(&spec, &mut cache).unwrap().len(), 2);
        assert_eq!(load_points(&spec, &mut cache).unwrap().len(), 2);
    }
}
