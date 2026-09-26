//! Property tests: rendering must never panic, and scaled data must land inside the plot area.

use proptest::prelude::*;
use termplt::plotting::{
    axes::{Axes, AxesPositioning},
    canvas::{BufferType, TerminalCanvas},
    colors,
    graph::Graph,
    grid_lines::GridLines,
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
        text_scale in prop_oneof![4 => 0usize..4, 1 => any::<usize>()],
        text_padding in prop_oneof![4 => 0usize..4, 1 => any::<usize>()],
        with_grid in any::<bool>(),
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
                TextStyle::new(colors::WHITE, text_scale, text_padding),
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

        // Ok or Err are both acceptable; a panic fails the test
        let _ = TerminalCanvas::new(width, height, colors::BLACK)
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
}
