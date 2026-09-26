mod cli;
mod data;
mod names;
mod series;

use clap::{CommandFactory, Parser, ValueEnum};
use cli::{Cli, LegendLoc};
use data::{Column, ColumnNames, Table};
use names::NameSource;
use series::{SeriesSpec, Source, Style};
use std::{
    error::Error,
    fs,
    io::{self, IsTerminal, Read},
    path::Path,
};
use termplt::{
    DEFAULT_PNG_SIZE, Plot,
    plotting::{colors, font::Font, text::DEFAULT_FONT_SIZE},
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
    let mut column_names = Vec::new();
    let mut plot = Plot::new()
        .background(series::parse_color(&cli.bg)?)
        .grid(!cli.no_grid)
        .legend(legend_shown(&cli, specs.len()));
    if let Some(loc) = cli.legend_loc {
        plot = plot.legend_location(loc.into());
    }
    let mut loaded = Vec::new();
    let mut name_sources = Vec::new();
    for (index, spec) in specs.iter().enumerate() {
        let (points, columns, y_column) = load_points(spec, &mut stdin_cache)?;
        let series = series::build_series(&points, &spec.style.or(&defaults), index)?;
        name_sources.push(NameSource {
            label: spec.label.clone(),
            path: match &spec.source {
                Source::File(path) => Some(path.clone()),
                Source::Inline(_) => None,
            },
            header: columns.y.clone(),
            column: y_column,
        });
        column_names.push(columns);
        loaded.push((series, points.len()));
    }
    let series_names = names::series_names(&name_sources);
    for (index, ((series, count), name)) in loaded.into_iter().zip(series_names).enumerate() {
        let series = match &name {
            Some(name) => series.with_label(name.clone()),
            None => series,
        };
        if cli.verbose {
            let label = name.map_or_else(|| "none".to_string(), |name| format!("{name:?}"));
            eprintln!(
                "[verbose] series {index}: {count} points from {}, label={label}, marker={:?}, \
                 line={:?}",
                specs[index].source.describe(),
                series.marker_style(),
                series.line_style()
            );
        }
        plot = plot.series(series);
    }
    if cli.verbose {
        if legend_shown(&cli, specs.len()) {
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

    // an image file needs no terminal; displaying one needs a terminal to size it and draw on
    let terminal = match &cli.output {
        Some(_) => None,
        None => {
            Some(Terminal::connect_with_log(cli.verbose, &mut io::stderr()).map_err(with_hint)?)
        }
    };

    let default_size = terminal
        .as_ref()
        .map_or(DEFAULT_PNG_SIZE, Terminal::default_plot_size);
    let (width, height) = (
        cli.width.unwrap_or(default_size.0),
        cli.height.unwrap_or(default_size.1),
    );
    let (font_size, font_size_source) = match (cli.font_size, &terminal) {
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
    }

    if cli.verbose {
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

/// Adds the --output alternative to errors that mean the terminal can't show images.
fn with_hint(e: termplt::Error) -> Box<dyn Error> {
    match e {
        termplt::Error::NotATerminal
        | termplt::Error::GraphicsUnsupported
        | termplt::Error::GraphicsRejected(_)
        | termplt::Error::TmuxPassthroughDisabled => {
            format!("{e}. Use --output plot.png to write an image file instead.").into()
        }
        termplt::Error::WindowSize(_) | termplt::Error::InvalidWindowSize { .. } => format!(
            "{e}. Set --width and --height, or use --output plot.png to write an image file \
             instead."
        )
        .into(),
        e => e.into(),
    }
}

/// Only PNG output is supported; catch other extensions before doing any work.
/// Reads a font file for --font.
fn load_font(path: &Path) -> Result<Font> {
    let data = fs::read(path).map_err(|e| format!("cannot read font '{}': {e}", path.display()))?;
    Font::from_bytes(data)
        .map_err(|_| format!("'{}' is not a TrueType or OpenType font", path.display()).into())
}

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

/// Whether to show the legend: `--no-legend` hides it; `--legend` or `--legend-loc` shows it;
/// otherwise it's shown for 2 or more series. The plot still draws one only when a series has
/// a name.
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
