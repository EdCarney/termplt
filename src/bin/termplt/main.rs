mod cli;
mod data;
mod follow;
mod names;
mod series;

use clap::{CommandFactory, Parser, ValueEnum};
use cli::{Cli, LegendLoc};
use data::{Column, ColumnNames, Table};
use follow::{Schedule, Stream};
use names::NameSource;
use series::{SeriesSpec, Source, Style};
use std::{
    error::Error,
    fs,
    io::{self, IsTerminal, Read},
    path::Path,
    sync::mpsc::RecvTimeoutError,
    time::{Duration, Instant},
};
use termplt::{
    DEFAULT_PNG_SIZE, LivePlot, Plot,
    plotting::{colors, font::Font, series::Series, text::DEFAULT_FONT_SIZE},
    terminal::{Image, Terminal},
};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

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

    if let Some(path) = &cli.output {
        check_output_path(path)?;
    }
    // a bad font fails before any data is read
    let font = cli.font.as_deref().map(load_font).transpose()?;

    let stdin_is_terminal = io::stdin().is_terminal();
    if cli.follow && stdin_is_terminal {
        return Err(
            "--follow plots data piped to stdin as it arrives, but stdin is a terminal; \
                    pipe the data in, e.g. `tail -f data.csv | termplt --follow`"
                .into(),
        );
    }
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
    // with --follow, stdin is read as it arrives; everything else is read now
    let mut stdin_cache = None;
    let mut loaded = Vec::new();
    for (index, spec) in specs.iter().enumerate() {
        let (points, columns, y_column) = if cli.follow && spec.source.is_stdin() {
            (Vec::new(), ColumnNames::default(), 0)
        } else {
            load_points(spec, &mut stdin_cache)?
        };
        let series = series::build_series(&points, &spec.style.or(&defaults), index)?;
        loaded.push(Loaded {
            series,
            columns,
            y_column,
        });
    }
    if cli.follow {
        return follow(&cli, &specs, loaded, font);
    }
    if limits_exclude_every_point(loaded.iter().map(|l| &l.series), cli.xlim, cli.ylim) {
        eprintln!(
            "warning: no data points lie within the axis limits (--xlim/--ylim); \
             drawing empty axes"
        );
    }
    let plot = build_plot(&cli, &specs, loaded)?;

    // an image file needs no terminal; displaying one needs a terminal to size it and draw on
    let terminal = match &cli.output {
        Some(_) => None,
        None => Some(connect(&cli)?),
    };
    let (plot, (width, height)) = fit(&cli, plot, terminal.as_ref(), font);

    match (&cli.output, terminal) {
        (Some(path), _) => {
            plot.save_png(path)
                .map_err(|e| format!("cannot write '{}': {e}", path.display()))?;
            if cli.verbose {
                eprintln!("[verbose] wrote {}", path.display());
            }
        }
        (None, Some(terminal)) => {
            let rgb = plot.render(width, height)?;
            let image = Image::png_from_rgb(&rgb, width, height)?;
            if cli.verbose {
                eprintln!(
                    "[verbose] sending {} bytes of PNG data ({} bytes uncompressed)",
                    image.payload_len(),
                    rgb.len()
                );
            }
            terminal.show(&image)?;
        }
        (None, None) => unreachable!("a terminal is connected whenever there is no --output"),
    }

    Ok(())
}

/// A series with its points and the column names it can be named after.
struct Loaded {
    series: Series,
    columns: ColumnNames,
    /// The y column's 1-based index.
    y_column: usize,
}

/// Whether `--xlim`/`--ylim` are set and no finite point of any series lies inside every limit
/// that is set (inclusive), so the plot would draw empty axes.
fn limits_exclude_every_point<'a>(
    series: impl IntoIterator<Item = &'a Series>,
    xlim: Option<(f64, f64)>,
    ylim: Option<(f64, f64)>,
) -> bool {
    let inside =
        |v: f64, lim: Option<(f64, f64)>| lim.is_none_or(|(min, max)| v >= min && v <= max);
    (xlim.is_some() || ylim.is_some())
        && !series.into_iter().any(|series| {
            series.data().iter().any(|p| {
                p.x.is_finite() && p.y.is_finite() && inside(p.x, xlim) && inside(p.y, ylim)
            })
        })
}

/// Builds the plot from the series, in the order of `specs`: the series named for the legend,
/// the legend, the axis names and the limits.
fn build_plot(cli: &Cli, specs: &[SeriesSpec], loaded: Vec<Loaded>) -> Result<Plot> {
    let mut plot = Plot::new()
        .background(series::parse_color(&cli.bg)?)
        .grid(!cli.no_grid)
        .legend(legend_shown(cli, specs.len()));
    if let Some(loc) = cli.legend_loc {
        plot = plot.legend_location(loc.into());
    }
    let name_sources: Vec<NameSource> = (specs.iter().zip(&loaded))
        .map(|(spec, loaded)| NameSource {
            label: spec.label.clone(),
            path: match &spec.source {
                Source::File(path) => Some(path.clone()),
                Source::Inline(_) => None,
            },
            header: loaded.columns.y.clone(),
            column: loaded.y_column,
        })
        .collect();
    let series_names = names::series_names(&name_sources);
    let mut column_names = Vec::new();
    for (index, (loaded, name)) in loaded.into_iter().zip(series_names).enumerate() {
        let series = match &name {
            Some(name) => loaded.series.with_label(name.clone()),
            None => loaded.series,
        };
        if cli.verbose {
            let label = name.map_or_else(|| "none".to_string(), |name| format!("{name:?}"));
            eprintln!(
                "[verbose] series {index}: {} points from {}, label={label}, marker={:?}, \
                 line={:?}",
                series.data().len(),
                specs[index].source.describe(),
                series.marker_style(),
                series.line_style()
            );
        }
        column_names.push(loaded.columns);
        plot = plot.series(series);
    }
    if cli.verbose {
        if legend_shown(cli, specs.len()) {
            let loc = cli.legend_loc.unwrap_or(LegendLoc::Best);
            let name = loc.to_possible_value().expect("no variant is skipped");
            eprintln!("[verbose] legend: on, {}", name.get_name());
        } else {
            eprintln!("[verbose] legend: off");
        }
    }
    // explicit names win; an empty one removes a name taken from the headers
    let names = data::axis_names(&column_names);
    if let Some(text) = cli.title.clone() {
        plot = plot.title(text);
    }
    if let Some(text) = cli.xlabel.clone().or(names.x) {
        plot = plot.x_label(text);
    }
    if let Some(text) = cli.ylabel.clone().or(names.y) {
        plot = plot.y_label(text);
    }
    if let Some((min, max)) = cli.xlim {
        plot = plot.x_limits(min, max);
    }
    if let Some((min, max)) = cli.ylim {
        plot = plot.y_limits(min, max);
    }
    Ok(plot)
}

/// Connects to the terminal, with hints for the errors that mean it can't show images.
fn connect(cli: &Cli) -> Result<Terminal> {
    Terminal::connect_with_log(cli.verbose, &mut io::stderr()).map_err(|e| with_hint(e, cli))
}

/// Sets the plot's size and text size (from the flags, else the terminal's, else the defaults
/// for --output) and its font, and returns it with its size.
fn fit(
    cli: &Cli,
    plot: Plot,
    terminal: Option<&Terminal>,
    font: Option<Font>,
) -> (Plot, (u32, u32)) {
    let default_size = terminal.map_or(DEFAULT_PNG_SIZE, Terminal::default_plot_size);
    let (width, height) = (
        cli.width.unwrap_or(default_size.0),
        cli.height.unwrap_or(default_size.1),
    );
    let (font_size, font_size_source) = match (cli.font_size, terminal) {
        (Some(px), _) => (px, "--font-size".to_string()),
        (None, Some(terminal)) => (
            terminal.text_size(),
            format!("terminal rows are {} px", terminal.window().pix_per_row),
        ),
        (None, None) => (DEFAULT_FONT_SIZE, "default for --output".to_string()),
    };
    let mut plot = plot.size(width, height).font_size(font_size);
    if let Some(font) = font {
        plot = plot.font(font);
    }
    if cli.verbose {
        let font_name = cli
            .font
            .as_ref()
            .map_or_else(|| "Go (built in)".to_string(), |p| p.display().to_string());
        eprintln!("[verbose] text: {font_size} px ({font_size_source}), font: {font_name}");
        eprintln!("[verbose] canvas: {width}x{height} pixels");
        match plot.canvas(width, height).get_drawable_limits() {
            Ok(area) => {
                let (w, h) = area.span();
                eprintln!(
                    "[verbose] plot area: {w}x{h} pixels at ({}, {})",
                    area.min().x,
                    area.min().y
                );
            }
            Err(e) => eprintln!("[verbose] plot area unavailable: {e}"),
        }
    }
    (plot, (width, height))
}

/// `--follow`: draws the plot in place once the first point arrives on stdin, and again as
/// more arrive, at most once per `--interval`, until stdin ends. Other output would move the
/// plot, so warnings about skipped rows wait for the end, and `--verbose` reports only up to
/// the first frame.
fn follow(
    cli: &Cli,
    specs: &[SeriesSpec],
    mut loaded: Vec<Loaded>,
    mut font: Option<Font>,
) -> Result<()> {
    let terminal = connect(cli)?;
    let interval = cli
        .interval
        .map_or(follow::DEFAULT_INTERVAL, Duration::from_millis);
    let window = cli.window.map(|n| usize::try_from(n).unwrap_or(usize::MAX));
    if cli.verbose {
        let window = window.map_or_else(|| "all".to_string(), |n| n.to_string());
        eprintln!(
            "[verbose] follow: interval {} ms, window {window}",
            interval.as_millis()
        );
    }

    let mut stream = Stream::new(
        (specs.iter().enumerate())
            .filter(|(_, spec)| spec.source.is_stdin())
            .map(|(index, spec)| (index, spec.x.clone(), spec.y.clone())),
    );
    let lines = follow::read_lines();
    let mut schedule = Schedule::new(interval);
    // the plot and its handle, from the first frame on
    let mut live: Option<(Plot, LivePlot)> = None;
    loop {
        let line = match schedule.wait(Instant::now()) {
            None => match lines.recv() {
                Ok(line) => Some(line),
                Err(_) => break,
            },
            Some(timeout) => match lines.recv_timeout(timeout) {
                Ok(line) => Some(line),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            },
        };
        if let Some(line) = line {
            let line = line.map_err(|e| format!("cannot read stdin: {e}"))?;
            for (index, point) in stream.push_line(&line)? {
                let series = match &mut live {
                    Some((plot, _)) => &mut plot.series_mut()[index],
                    None => &mut loaded[index].series,
                };
                series.push(point.x, point.y);
                if let Some(n) = window {
                    series.keep_last(n);
                }
                schedule.points_arrived();
            }
        }
        if !schedule.due(Instant::now()) {
            continue;
        }
        match &mut live {
            Some((plot, handle)) => handle.update(plot)?,
            None => {
                // the stream's columns are known by its first point, and with them the names
                for (index, loaded) in loaded.iter_mut().enumerate() {
                    if let Some(columns) = stream.columns(index) {
                        loaded.columns = columns.names.clone();
                        loaded.y_column = columns.y_column;
                    }
                }
                let plot = build_plot(cli, specs, std::mem::take(&mut loaded))?;
                let (plot, _) = fit(cli, plot, Some(&terminal), font.take());
                let handle = plot.show_live_in(&terminal)?;
                live = Some((plot, handle));
            }
        }
        schedule.drawn(Instant::now());
    }

    // the end of stdin: the points that arrived since the last frame, then the warnings
    if schedule.pending()
        && let Some((plot, handle)) = &mut live
    {
        handle.update(plot)?;
    }
    stream.finish()?;
    if let Some(warning) = stream.skipped() {
        eprintln!("{warning}");
    }
    if live.is_none() {
        return Err("no data points found in stdin".into());
    }
    Ok(())
}

/// Adds the --output alternative to errors that mean the terminal can't show images (except
/// with --follow, which only draws in the terminal).
fn with_hint(e: termplt::Error, cli: &Cli) -> Box<dyn Error> {
    match e {
        termplt::Error::NotATerminal
        | termplt::Error::GraphicsUnsupported
        | termplt::Error::GraphicsRejected(_)
        | termplt::Error::TmuxPassthroughDisabled
            if !cli.follow =>
        {
            format!("{e}. Use --output plot.png to write an image file instead.").into()
        }
        termplt::Error::WindowSize(_) | termplt::Error::InvalidWindowSize { .. } if cli.follow => {
            format!("{e}. Set --width and --height.").into()
        }
        termplt::Error::WindowSize(_) | termplt::Error::InvalidWindowSize { .. } => format!(
            "{e}. Set --width and --height, or use --output plot.png to write an image file \
             instead."
        )
        .into(),
        e => e.into(),
    }
}

/// Reads a font file for --font.
fn load_font(path: &Path) -> Result<Font> {
    let data = fs::read(path).map_err(|e| format!("cannot read font '{}': {e}", path.display()))?;
    Font::from_bytes(data)
        .map_err(|_| format!("'{}' is not a TrueType or OpenType font", path.display()).into())
}

/// Only PNG output is supported; catch other extensions before doing any work.
fn check_output_path(path: &Path) -> Result<()> {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("png") => Ok(()),
        _ => Err(format!(
            "cannot write '{}': only PNG output is supported; use a .png file name",
            path.display()
        )
        .into()),
    }
}

/// Collects the series to plot, in order: FILE arguments (one series per y column), --data,
/// then --series. Stdin is used when it is piped and no other data is given, and always with
/// --follow.
fn collect_specs(cli: &Cli, stdin_is_piped: bool) -> Result<Vec<SeriesSpec>> {
    let x = cli.x_col.as_deref().map(Column::parse).transpose()?;
    let y_cols = cli
        .y_col
        .iter()
        .map(|c| Column::parse(c))
        .collect::<Result<Vec<_>>>()?;

    let series_specs = (cli.series.iter())
        .map(|spec| series::parse_spec(spec))
        .collect::<Result<Vec<_>>>()?;

    let mut files: Vec<&String> = cli.files.iter().chain(&cli.data_file).collect();
    let stdin = "-".to_string();
    let only_stdin =
        files.is_empty() && cli.data.is_empty() && cli.series.is_empty() && stdin_is_piped;
    // --follow always reads stdin, as a FILE unless it is one already
    let reads_stdin = files.iter().any(|file| *file == "-")
        || series_specs.iter().any(|spec| spec.source.is_stdin());
    if only_stdin || (cli.follow && !reads_stdin) {
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
    for mut spec in series_specs {
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

/// Whether to show the legend: `--no-legend` hides it; `--legend` or `--legend-loc` shows it
/// (the last of them given wins, so the overridden flags are already cleared); otherwise it's
/// shown for 2 or more series. The plot still draws one only when a series has a name.
fn legend_shown(cli: &Cli, series: usize) -> bool {
    !cli.no_legend && (cli.legend || cli.legend_loc.is_some() || series >= 2)
}

/// Reads and parses a series' points, skipping (with a warning) rows with missing values and
/// points with NaN or infinite coordinates. Stdin is read at most once and shared.
fn load_points(
    spec: &SeriesSpec,
    stdin_cache: &mut Option<String>,
) -> Result<(
    Vec<termplt::plotting::point::Point<f64>>,
    ColumnNames,
    usize,
)> {
    let source = spec.source.describe();
    let (points, skipped_missing, names, y_column) = match &spec.source {
        Source::Inline(s) => {
            let points = data::parse_inline(s).map_err(|e| format!("{source}: {e}"))?;
            (points, 0, ColumnNames::default(), 0)
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
            (
                parsed.points,
                parsed.skipped_missing,
                parsed.names,
                parsed.y_column,
            )
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
    Ok((points, names, y_column))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_points_reports_the_y_column() {
        let mut spec = SeriesSpec::new(Source::File("-".into()));
        spec.y = Some(Column::Name("b".into()));
        let mut cache = Some("t,a,b\n1,2,3\n".to_string());
        let (_, columns, y_column) = load_points(&spec, &mut cache).unwrap();
        assert_eq!((columns.y.as_deref(), y_column), (Some("b"), 3));
    }

    fn cli(args: &[&str]) -> Cli {
        Cli::try_parse_from(std::iter::once("termplt").chain(args.iter().copied())).unwrap()
    }

    fn series_of(points: &[(f64, f64)]) -> Series {
        Series::new(
            &points
                .iter()
                .map(|&(x, y)| termplt::prelude::Point::new(x, y))
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn limits_exclude_every_point_only_when_limits_are_set_and_nothing_is_inside() {
        let data = [series_of(&[(1.0, 1.0), (2.0, 4.0), (f64::NAN, 2.0)])];
        let excluded = |xlim, ylim| limits_exclude_every_point(&data, xlim, ylim);
        assert!(!excluded(None, None));
        assert!(!excluded(Some((0.0, 10.0)), None));
        assert!(excluded(Some((100.0, 200.0)), None));
        assert!(excluded(None, Some((10.0, 20.0))));
        // each point is inside one limit, but none is inside both
        assert!(excluded(Some((1.0, 1.0)), Some((4.0, 4.0))));
        // limits are inclusive
        assert!(!excluded(Some((2.0, 3.0)), Some((4.0, 4.0))));
        // non-finite points never count; no points at all is excluded when limits are set
        let nan = [series_of(&[(f64::NAN, f64::NAN)]), series_of(&[])];
        assert!(limits_exclude_every_point(&nan, Some((0.0, 1.0)), None));
        // one series with a point inside is enough
        let two = [series_of(&[(50.0, 0.0)]), series_of(&[(150.0, 0.0)])];
        assert!(!limits_exclude_every_point(
            &two,
            Some((100.0, 200.0)),
            None
        ));
    }

    #[test]
    fn the_legend_shows_for_two_or_more_series_unless_told_otherwise() {
        assert!(!legend_shown(&cli(&[]), 1));
        assert!(legend_shown(&cli(&[]), 2));
        assert!(legend_shown(&cli(&["--legend"]), 1));
        assert!(legend_shown(&cli(&["--legend-loc", "upper-left"]), 1));
        assert!(!legend_shown(&cli(&["--no-legend"]), 2));
        assert!(!legend_shown(
            &cli(&["--legend-loc", "center", "--no-legend"]),
            2
        ));
        assert!(legend_shown(
            &cli(&["--no-legend", "--legend-loc", "center"]),
            1
        ));
        assert!(!legend_shown(&cli(&["--legend", "--no-legend"]), 1));
        assert!(legend_shown(&cli(&["--no-legend", "--legend"]), 1));
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
    fn follow_always_reads_stdin() {
        let sources = |args: &[&str]| -> Vec<Source> {
            let specs = collect_specs(&cli(args), true).unwrap();
            specs.into_iter().map(|s| s.source).collect()
        };
        let stdin = Source::File("-".into());
        assert_eq!(sources(&["-f"]), std::slice::from_ref(&stdin));
        // after the FILE arguments, before --data and --series
        assert_eq!(
            sources(&["-f", "a.csv", "-d", "(1,2)"]),
            [
                Source::File("a.csv".into()),
                stdin.clone(),
                Source::Inline("(1,2)".into())
            ]
        );
        // an explicit '-' is the same source, as a FILE or in a --series
        assert_eq!(
            sources(&["-f", "-", "a.csv"]),
            [stdin.clone(), Source::File("a.csv".into())]
        );
        assert_eq!(
            sources(&["-f", "-s", "file=-,color=red"]),
            std::slice::from_ref(&stdin)
        );
        // one series per y column, as for any file
        let specs = collect_specs(&cli(&["-f", "-x", "time", "-y", "t,h"]), true).unwrap();
        assert_eq!(specs.len(), 2);
        assert!(specs.iter().all(|s| s.source == stdin));
        assert_eq!(specs[1].y, Some(Column::Name("h".into())));
    }

    #[test]
    fn follow_needs_no_help_text_without_other_data() {
        // stdin is always a source, so there is always something to plot
        assert_eq!(collect_specs(&cli(&["--follow"]), true).unwrap().len(), 1);
        assert_eq!(collect_specs(&cli(&[]), true).unwrap().len(), 1);
    }

    #[test]
    fn legacy_data_file_flag_is_a_file() {
        let specs = collect_specs(&cli(&["--data_file", "a.csv"]), false).unwrap();
        assert_eq!(specs[0].source, Source::File("a.csv".into()));
    }

    #[test]
    fn output_must_be_png() {
        assert!(check_output_path(Path::new("plot.png")).is_ok());
        assert!(check_output_path(Path::new("out/Plot.PNG")).is_ok());
        let err = check_output_path(Path::new("plot.jpg")).unwrap_err();
        assert!(err.to_string().contains("only PNG output is supported"));
        assert!(check_output_path(Path::new("plot")).is_err());
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
        let points = load_points(&spec, &mut None).unwrap().0;
        assert_eq!(points.len(), 2);
        let spec = SeriesSpec::new(Source::Inline("(nan,1),(2,-inf)".into()));
        assert!(load_points(&spec, &mut None).is_err());
    }

    #[test]
    fn load_points_reuses_cached_stdin() {
        let spec = SeriesSpec::new(Source::File("-".into()));
        let mut cache = Some("x,y\n1,2\n3,4\n".to_string());
        assert_eq!(load_points(&spec, &mut cache).unwrap().0.len(), 2);
        assert_eq!(load_points(&spec, &mut cache).unwrap().0.len(), 2);
    }
}
