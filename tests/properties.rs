//! Property tests: rendering must never panic, and scaled data must land inside the plot area.

use proptest::prelude::*;
use termplt::plotting::{
    axes::{Axes, AxesPositioning},
    canvas::{BufferType, TerminalCanvas},
    colors,
    graph::Graph,
    grid_lines::GridLines,
    legend::LegendLocation,
    limits::Limits,
    line::LineStyle,
    marker::MarkerStyle,
    point::Point,
    series::Series,
    text::TextStyle,
};

/// Any f64, weighted towards ordinary values but including NaN, infinities and extremes.
fn any_coord() -> impl Strategy<Value = f64> + Clone {
    prop_oneof![
        8 => -1e6..1e6f64,
        1 => prop::num::f64::ANY,
        1 => prop_oneof![
            Just(0.0),
            Just(f64::NAN),
            Just(f64::INFINITY),
            Just(f64::NEG_INFINITY),
            Just(f64::MAX),
            Just(f64::MIN),
            Just(f64::MIN_POSITIVE),
        ],
    ]
}

fn points(coord: impl Strategy<Value = f64> + Clone) -> impl Strategy<Value = Vec<Point<f64>>> {
    prop::collection::vec(
        (coord.clone(), coord).prop_map(|(x, y)| Point::new(x, y)),
        0..40,
    )
}

/// A small value below `typical` most of the time, sometimes anything up to `u32::MAX`, so
/// layout arithmetic is exercised at its limits.
fn size(typical: u32) -> impl Strategy<Value = u32> + Clone {
    prop_oneof![
        8 => 0..typical,
        1 => any::<u32>(),
        1 => Just(u32::MAX),
    ]
}

fn marker() -> impl Strategy<Value = MarkerStyle> {
    size(6).prop_flat_map(|size| {
        prop_oneof![
            Just(MarkerStyle::None),
            Just(MarkerStyle::FilledSquare {
                size,
                color: colors::RED
            }),
            Just(MarkerStyle::HollowSquare {
                size,
                color: colors::RED
            }),
            Just(MarkerStyle::FilledCircle {
                size,
                color: colors::RED
            }),
            Just(MarkerStyle::HollowCircle {
                size,
                color: colors::RED
            }),
        ]
    })
}

fn line() -> impl Strategy<Value = Option<LineStyle>> {
    prop::option::of((size(4), any::<bool>()).prop_map(|(thickness, dashed)| {
        if dashed {
            LineStyle::Dashed {
                color: colors::BLUE,
                thickness,
            }
        } else {
            LineStyle::Solid {
                color: colors::BLUE,
                thickness,
            }
        }
    }))
}

fn buffer() -> impl Strategy<Value = BufferType> {
    prop_oneof![
        size(40).prop_map(BufferType::Uniform),
        (size(40), size(40)).prop_map(|(t, b)| BufferType::TopBottom(t, b)),
        (size(40), size(40)).prop_map(|(l, r)| BufferType::LeftRight(l, r)),
        (size(40), size(40), size(40), size(40))
            .prop_map(|(t, b, l, r)| BufferType::TopBottomLeftRight(t, b, l, r)),
    ]
}

fn series() -> impl Strategy<Value = Series> {
    (points(any_coord()), marker(), line()).prop_map(|(points, marker, line)| {
        let series = Series::new(&points).with_marker_style(marker);
        match line {
            Some(line) => series.with_line_style(line),
            None => series,
        }
    })
}

/// Titles and axis names: absent, blank, any printable text, or far too long.
fn text() -> impl Strategy<Value = Option<String>> {
    prop::option::of(prop_oneof![
        4 => "\\PC{0,40}",
        1 => "[a-z ]{150,400}",
        1 => Just("\n \n".to_string()),
    ])
}

/// Legend labels: any printable text, blank, or far too long.
fn label() -> impl Strategy<Value = String> {
    prop_oneof![
        4 => "\\PC{0,40}",
        1 => "[a-z ]{50,200}",
        1 => Just(String::new()),
        1 => Just(" \n ".to_string()),
    ]
}

fn location() -> impl Strategy<Value = LegendLocation> {
    use LegendLocation::*;
    prop::sample::select(vec![
        Best,
        UpperRight,
        UpperLeft,
        LowerLeft,
        LowerRight,
        Right,
        CenterLeft,
        CenterRight,
        LowerCenter,
        UpperCenter,
        Center,
    ])
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn draw_never_panics(
        series in prop::collection::vec(series(), 0..4),
        width in 0u32..300,
        height in 0u32..300,
        buffer in buffer(),
        with_axes in any::<bool>(),
        axis_thickness in size(3),
        text_size in prop_oneof![4 => 0u32..40, 1 => any::<u32>()],
        font_size in prop_oneof![4 => 1u32..40, 1 => any::<u32>()],
        with_grid in any::<bool>(),
        title in text(),
        x_label in text(),
        y_label in text(),
        x_limits in prop::option::of((any_coord(), any_coord())),
        y_limits in prop::option::of((any_coord(), any_coord())),
    ) {
        let mut graph = Graph::new();
        for s in series {
            graph = graph.with_series(s);
        }
        if with_axes {
            graph = graph.with_axes(Axes::new(
                AxesPositioning::XY(LineStyle::solid(colors::WHITE, axis_thickness)),
                TextStyle::new(colors::WHITE, text_size),
            ));
        }
        if with_grid {
            graph = graph.with_grid_lines(GridLines::XY(LineStyle::default()));
        }
        if let Some((min, max)) = x_limits {
            graph = graph.with_x_limits(min, max);
        }
        if let Some((min, max)) = y_limits {
            graph = graph.with_y_limits(min, max);
        }

        if let Some(text) = title {
            graph = graph.with_title(text);
        }
        if let Some(text) = x_label {
            graph = graph.with_x_label(text);
        }
        if let Some(text) = y_label {
            graph = graph.with_y_label(text);
        }

        // Ok or Err are both acceptable; a panic fails the test
        let _ = TerminalCanvas::new(width, height, colors::BLACK)
            .with_font_size(font_size)
            .with_buffer(buffer)
            .with_graph(graph)
            .draw();
    }

    #[test]
    fn scaled_points_lie_within_new_limits(
        data in points(-1e9..1e9f64).prop_filter("need data", |p| !p.is_empty()),
        min_x in 0.0..500.0f64,
        min_y in 0.0..500.0f64,
        w in 1.0..1000.0f64,
        h in 1.0..1000.0f64,
    ) {
        let new_limits = Limits::new(Point::new(min_x, min_y), Point::new(min_x + w, min_y + h));
        let scaled = Graph::new()
            .with_series(Series::new(&data))
            .scale(new_limits.clone())
            .unwrap();

        let tolerance = 1e-6 * (1.0 + w.max(h));
        for p in scaled.data()[0].data() {
            prop_assert!(
                p.x >= min_x - tolerance && p.x <= min_x + w + tolerance,
                "x {} outside {}..{}", p.x, min_x, min_x + w
            );
            prop_assert!(
                p.y >= min_y - tolerance && p.y <= min_y + h + tolerance,
                "y {} outside {}..{}", p.y, min_y, min_y + h
            );
        }
    }

    #[test]
    fn the_legend_stays_inside_the_plot_area(
        series in prop::collection::vec(series(), 1..5),
        labels in prop::collection::vec(label(), 1..5),
        width in 1u32..400,
        height in 1u32..400,
        font_size in prop_oneof![4 => 1u32..40, 1 => 1u32..=400],
        location in location(),
    ) {
        let graph = |labeled: bool| {
            let mut graph = Graph::new()
                .with_axes(Axes::new(
                    AxesPositioning::XY(LineStyle::solid(colors::WHITE, 1)),
                    TextStyle::with_color(colors::WHITE),
                ))
                .with_legend_location(location);
            for (s, label) in series.iter().zip(labels.iter().cycle()) {
                let s = if labeled { s.clone().with_label(label.clone()) } else { s.clone() };
                graph = graph.with_series(s);
            }
            graph
        };
        let canvas = |graph: Graph| {
            TerminalCanvas::new(width, height, colors::BLACK)
                .with_buffer(BufferType::Uniform(4))
                .with_font_size(font_size)
                .with_graph(graph)
        };
        // all three run before any result is looked at: a panic fails the test, errors are fine
        let (plain, labeled, plot) = (
            canvas(graph(false)).draw(),
            canvas(graph(true)).draw(),
            canvas(graph(false)).get_drawable_limits(),
        );
        let (Ok(plain), Ok(labeled), Ok(plot)) = (plain, labeled, plot) else {
            return Ok(());
        };
        let (plain, labeled) = (plain.into_bytes(), labeled.into_bytes());
        for (i, (a, b)) in plain.chunks(3).zip(labeled.chunks(3)).enumerate() {
            if a != b {
                let (x, y) = (i as u32 % width, height - 1 - i as u32 / width);
                prop_assert!(
                    plot.contains(&Point::new(x, y)),
                    "pixel ({}, {}) changed outside the plot area {:?}", x, y, plot
                );
            }
        }
    }
}
