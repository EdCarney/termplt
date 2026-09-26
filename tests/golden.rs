//! Golden-image tests: render fixed scenes and compare them with PNG snapshots in
//! `tests/snapshots/`.
//!
//! To create or update snapshots after an intentional rendering change, run
//! `TERMPLT_UPDATE_SNAPSHOTS=1 cargo test --test golden` and review the PNGs before committing.
//! On a mismatch, the rendered image is written to `target/snapshots/<name>.actual.png`.

use std::path::PathBuf;
use termplt::plotting::{
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
};

/// Fraction of pixels allowed to differ, to absorb last-bit differences in platform math
/// libraries (e.g. the trigonometry used for circle markers).
const TOLERANCE: f64 = 0.001;

fn check(name: &str, width: u32, height: u32, canvas: TerminalCanvas) {
    let actual = canvas.draw().expect("scene should draw").get_bytes();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join("tests/snapshots").join(format!("{name}.png"));

    if std::env::var_os("TERMPLT_UPDATE_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::save_buffer(&path, &actual, width, height, image::ColorType::Rgb8).unwrap();
        return;
    }

    let expected = image::open(&path)
        .unwrap_or_else(|e| {
            panic!(
                "missing snapshot {} ({e}); run with TERMPLT_UPDATE_SNAPSHOTS=1 to create it",
                path.display()
            )
        })
        .to_rgb8();
    assert_eq!(
        expected.dimensions(),
        (width, height),
        "snapshot {name} has different dimensions"
    );

    let differing = expected
        .as_raw()
        .chunks(3)
        .zip(actual.chunks(3))
        .filter(|(e, a)| e != a)
        .count();
    let allowed = (TOLERANCE * (width * height) as f64) as usize;
    if differing > allowed {
        let out = root
            .join("target/snapshots")
            .join(format!("{name}.actual.png"));
        std::fs::create_dir_all(out.parent().unwrap()).unwrap();
        image::save_buffer(&out, &actual, width, height, image::ColorType::Rgb8).unwrap();
        panic!(
            "snapshot {name}: {differing} pixels differ (allowed {allowed}); rendered image written to {}",
            out.display()
        );
    }
}

fn axes() -> Axes {
    Axes::new(
        AxesPositioning::XY(LineStyle::Solid {
            color: colors::GRAY,
            thickness: 1,
        }),
        TextStyle::with_color(colors::WHITE),
    )
}

fn grid() -> GridLines {
    GridLines::XY(LineStyle::Solid {
        color: colors::DIM_GRAY,
        thickness: 0,
    })
}

/// Points of y = f(x) for x = start, start + step, ... (n points), using only arithmetic so
/// the data is identical on every platform.
fn curve(n: usize, start: f64, step: f64, f: impl Fn(f64) -> f64) -> Vec<Point<f64>> {
    (0..n)
        .map(|i| {
            let x = start + i as f64 * step;
            Point::new(x, f(x))
        })
        .collect()
}

#[test]
fn line_and_scatter_series() {
    let (w, h) = (320, 240);
    let cubic = Series::new(&curve(21, -2.0, 0.2, |x| x * x * x - 2.0 * x))
        .with_marker_style(MarkerStyle::FilledCircle {
            size: 2,
            color: colors::DODGER_BLUE,
        })
        .with_line_style(LineStyle::Solid {
            color: colors::DODGER_BLUE,
            thickness: 0,
        });
    let scatter = Series::new(&curve(9, -2.0, 0.5, |x| 1.5 - x * 0.5)).with_marker_style(
        MarkerStyle::HollowSquare {
            size: 3,
            color: colors::ORANGE,
        },
    );
    let canvas = TerminalCanvas::new(w, h, colors::BLACK)
        .with_buffer(BufferType::Uniform(8))
        .with_graph(
            Graph::new()
                .with_series(cubic)
                .with_series(scatter)
                .with_axes(axes())
                .with_grid_lines(grid()),
        );
    check("line_and_scatter_series", w, h, canvas);
}

#[test]
fn thick_and_dashed_lines_without_markers() {
    let (w, h) = (320, 240);
    let thick = Series::new(&curve(11, 0.0, 1.0, |x| x * x))
        .with_marker_style(MarkerStyle::None)
        .with_line_style(LineStyle::Solid {
            color: colors::LIME,
            thickness: 2,
        });
    let dashed = Series::new(&curve(11, 0.0, 1.0, |x| 10.0 * x))
        .with_marker_style(MarkerStyle::None)
        .with_line_style(LineStyle::Dashed {
            color: colors::MAGENTA,
            thickness: 0,
        });
    let canvas = TerminalCanvas::new(w, h, colors::BLACK)
        .with_buffer(BufferType::Uniform(8))
        .with_graph(
            Graph::new()
                .with_series(thick)
                .with_series(dashed)
                .with_axes(axes())
                .with_grid_lines(grid()),
        );
    check("thick_and_dashed_lines_without_markers", w, h, canvas);
}

#[test]
fn single_point_is_centered() {
    let (w, h) = (200, 200);
    let series =
        Series::new(&[Point::new(3.0, 3.0)]).with_marker_style(MarkerStyle::FilledSquare {
            size: 3,
            color: colors::RED,
        });
    let canvas = TerminalCanvas::new(w, h, colors::BLACK)
        .with_buffer(BufferType::Uniform(8))
        .with_graph(
            Graph::new()
                .with_series(series)
                .with_axes(axes())
                .with_grid_lines(grid()),
        );
    check("single_point_is_centered", w, h, canvas);
}

#[test]
fn explicit_limits_clip_data() {
    let (w, h) = (320, 200);
    let series = Series::new(&curve(41, -5.0, 0.5, |x| x * x - 10.0))
        .with_marker_style(MarkerStyle::FilledSquare {
            size: 1,
            color: colors::CYAN,
        })
        .with_line_style(LineStyle::Solid {
            color: colors::CYAN,
            thickness: 0,
        });
    let far_away = Series::new(&[Point::new(100.0, 100.0), Point::new(200.0, 50.0)]);
    let canvas = TerminalCanvas::new(w, h, colors::BLACK)
        .with_buffer(BufferType::Uniform(8))
        .with_graph(
            Graph::new()
                .with_series(series)
                .with_series(far_away)
                .with_x_limits(-4.0, 4.0)
                .with_y_limits(-10.0, 5.0)
                .with_axes(axes())
                .with_grid_lines(grid()),
        );
    check("explicit_limits_clip_data", w, h, canvas);
}

#[test]
fn scientific_notation_ticks() {
    let (w, h) = (320, 240);
    let series = Series::new(&curve(11, 0.0, 2.5e5, |x| x * 1e-11))
        .with_marker_style(MarkerStyle::FilledSquare {
            size: 1,
            color: colors::GOLD,
        })
        .with_line_style(LineStyle::Solid {
            color: colors::GOLD,
            thickness: 0,
        });
    let canvas = TerminalCanvas::new(w, h, colors::BLACK)
        .with_buffer(BufferType::Uniform(8))
        .with_graph(
            Graph::new()
                .with_series(series)
                .with_axes(axes())
                .with_grid_lines(grid()),
        );
    check("scientific_notation_ticks", w, h, canvas);
}

#[test]
fn large_offset_ticks() {
    // timestamps on x, and y values near 1e15 that f64 can only store 0.125 apart: both axes
    // are labeled relative to an offset (+1.7e9, +1e15) with evenly spaced ticks
    let (w, h) = (320, 240);
    let points: Vec<_> = (0..=15)
        .map(|i| Point::new(1_700_000_000.0 + 10.0 * i as f64, 1e15 + 0.1 * i as f64))
        .collect();
    let series = Series::new(&points)
        .with_marker_style(MarkerStyle::FilledSquare {
            size: 1,
            color: colors::GOLD,
        })
        .with_line_style(LineStyle::Solid {
            color: colors::GOLD,
            thickness: 0,
        });
    let canvas = TerminalCanvas::new(w, h, colors::BLACK)
        .with_buffer(BufferType::Uniform(8))
        .with_graph(
            Graph::new()
                .with_series(series)
                .with_axes(axes())
                .with_grid_lines(grid()),
        );
    check("large_offset_ticks", w, h, canvas);
}

#[test]
fn label_color_matching_background_is_replaced() {
    // white labels on a white canvas are drawn in black instead
    let (w, h) = (240, 180);
    let series = Series::new(&curve(6, -1.0, 0.5, |x| -x))
        .with_marker_style(MarkerStyle::FilledSquare {
            size: 2,
            color: colors::RED,
        })
        .with_line_style(LineStyle::Solid {
            color: colors::RED,
            thickness: 1,
        });
    let canvas = TerminalCanvas::new(w, h, colors::WHITE)
        .with_buffer(BufferType::Uniform(8))
        .with_graph(
            Graph::new()
                .with_series(series)
                .with_axes(axes())
                .with_grid_lines(GridLines::XY(LineStyle::Solid {
                    color: colors::LIGHT_GRAY,
                    thickness: 0,
                })),
        );
    check("label_color_matching_background_is_replaced", w, h, canvas);
}
