use std::f32;
use termplt::{
    get_window_size,
    kitty_graphics::{
        ctrl_seq::{PixelFormat, Transmission},
        png_imgs, rgb_imgs, rgba_imgs,
    },
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
        text::{Label, Text, TextPositioning, TextStyle},
    },
    terminal_commands::{
        csi_cmds::{self, CsiCommand},
        images::{Image, PositioningType},
        responses::TermCommand,
    },
};

fn main() {
    test_text();
    test_graphing();
    let win_sz = get_window_size().unwrap();
    println!("{win_sz:?}");
}

fn test_text() {
    let txt_1 = Text::from_number(-0.008911, 2, TextStyle::new(colors::RED, 1, 1));
    let lab_1 = Label::new(txt_1, TextPositioning::Centered(Point::new(100, 50)));

    let txt_2 = Text::from_number(-112.1233381, 3, TextStyle::new(colors::BLUE, 1, 1));
    let lab_2 = Label::new(txt_2, TextPositioning::Centered(Point::new(100, 100)));

    let width = 200;
    let height = 200;
    let bytes = TerminalCanvas::<u32>::new(width, height, colors::WHITE)
        .with_buffer(BufferType::Uniform(10))
        .with_label(lab_1)
        .with_label(lab_2)
        .draw()
        .unwrap()
        .get_bytes();
    Image::new(
        PixelFormat::Rgb { width, height },
        Transmission::Direct(bytes),
    )
    .unwrap()
    .display()
    .unwrap();
    println!();
}

fn test_graphing() {
    let num_points = 200;
    let points_x2 = (-num_points / 2..=num_points / 2)
        .map(|x| Point::new(x, x * x))
        .collect::<Vec<Point<_>>>();
    let points_x3 = (-num_points / 2..=num_points / 2)
        .map(|x| Point::new(x, x * x * x))
        .collect::<Vec<Point<_>>>();

    let num_points = 100;
    let points_sin = (0..=num_points)
        .map(|x| {
            let x = (x as f32) * (2. * f32::consts::PI / (num_points as f32));
            Point::new(x, x.sin())
        })
        .collect::<Vec<Point<_>>>();

    // draw_graph_style_1(&points_x2, None, None);
    draw_graph_style_2(&points_sin);
    draw_graph_style_3(&points_x3, None, None);
}

fn draw_graph_style_1(
    data: &[Point<i32>],
    x_lim_maybe: Option<(i32, i32)>,
    y_lim_maybe: Option<(i32, i32)>,
) {
    let width = 500;
    let height = 500;
    let mut graph = Graph::new()
        .with_series(Series::new(data).with_line_style(LineStyle::Solid {
            color: colors::BLUE,
            thickness: 0,
        }))
        .with_axes(Axes::new(
            AxesPositioning::XY(LineStyle::Solid {
                color: colors::BLACK,
                thickness: 1,
            }),
            TextStyle::with_color(colors::WHITE),
        ))
        .with_grid_lines(GridLines::XY(LineStyle::Solid {
            color: colors::BLACK,
            thickness: 0,
        }));

    if let Some((min, max)) = x_lim_maybe {
        graph = graph.with_x_limits(min, max);
    }

    if let Some((min, max)) = y_lim_maybe {
        graph = graph.with_y_limits(min, max);
    }

    let bytes = TerminalCanvas::new(width, height, colors::WHITE)
        .with_buffer(BufferType::Uniform(10))
        .with_graph(graph)
        .draw()
        .unwrap()
        .get_bytes();
    Image::new(
        PixelFormat::Rgb { width, height },
        Transmission::Direct(bytes),
    )
    .unwrap()
    .display()
    .unwrap();
    println!();
}

fn draw_graph_style_2(data: &[Point<f32>]) {
    let width = 900;
    let height = 900;
    let bytes = TerminalCanvas::new(width, height, colors::BLACK)
        .with_buffer(BufferType::Uniform(100))
        .with_graph(
            Graph::new()
                .with_series(
                    Series::new(data)
                        .with_marker_style(MarkerStyle::FilledCircle {
                            size: 3,
                            color: colors::LIME,
                        })
                        .with_line_style(LineStyle::Solid {
                            color: colors::LIME,
                            thickness: 0,
                        }),
                )
                .with_series(
                    Series::new(data)
                        .with_marker_style(MarkerStyle::HollowCircle {
                            size: 3,
                            color: colors::RED,
                        })
                        .with_line_style(LineStyle::Solid {
                            color: colors::RED,
                            thickness: 0,
                        })
                        + Point::new(f32::consts::FRAC_PI_4, 0.),
                )
                .with_series(
                    Series::new(data)
                        .with_marker_style(MarkerStyle::FilledSquare {
                            size: 3,
                            color: colors::CYAN,
                        })
                        .with_line_style(LineStyle::Solid {
                            color: colors::CYAN,
                            thickness: 0,
                        })
                        + Point::new(2. * f32::consts::FRAC_PI_4, 0.),
                )
                .with_series(
                    Series::new(data)
                        .with_marker_style(MarkerStyle::HollowSquare {
                            size: 3,
                            color: colors::ORANGE,
                        })
                        .with_line_style(LineStyle::Solid {
                            color: colors::ORANGE,
                            thickness: 0,
                        })
                        + Point::new(3. * f32::consts::FRAC_PI_4, 0.),
                )
                .with_axes(Axes::new(
                    AxesPositioning::XY(LineStyle::Solid {
                        color: colors::GHOST_WHITE,
                        thickness: 1,
                    }),
                    TextStyle::with_color(colors::WHITE),
                ))
                .with_grid_lines(GridLines::XY(LineStyle::Solid {
                    color: colors::GRAY,
                    thickness: 0,
                })),
        )
        .draw()
        .unwrap()
        .get_bytes();

    let format = PixelFormat::Rgb { width, height };
    let transmission = Transmission::Direct(bytes);
    Image::new(format, transmission).unwrap().display().unwrap();
    println!();
}

fn draw_graph_style_3(
    data: &[Point<i32>],
    x_lim_maybe: Option<(i32, i32)>,
    y_lim_maybe: Option<(i32, i32)>,
) {
    let width = 700;
    let height = 700;
    let mut graph = Graph::new()
        .with_series(Series::new(data).with_line_style(LineStyle::Solid {
            color: colors::BLUE,
            thickness: 0,
        }))
        .with_axes(Axes::new(
            AxesPositioning::XY(LineStyle::Solid {
                color: colors::BLACK,
                thickness: 1,
            }),
            TextStyle::default(),
        ))
        .with_grid_lines(GridLines::XY(LineStyle::Solid {
            color: colors::BLACK,
            thickness: 0,
        }));

    if let Some((min, max)) = x_lim_maybe {
        graph = graph.with_x_limits(min, max);
    }

    if let Some((min, max)) = y_lim_maybe {
        graph = graph.with_y_limits(min, max);
    }

    let bytes = TerminalCanvas::new(width, height, colors::WHITE)
        .with_buffer(BufferType::Uniform(50))
        .with_graph(graph)
        .draw()
        .unwrap()
        .get_bytes();
    Image::new(
        PixelFormat::Rgb { width, height },
        Transmission::Direct(bytes),
    )
    .unwrap()
    .display()
    .unwrap();
    println!();
}

fn test_new_terminal_cmds() {
    csi_cmds::clear_screen().unwrap();

    rgb_imgs::print_square(50, colors::RED).unwrap();
    println!();

    let position = PositioningType::ExactPixel { x: 10, y: 10 };
    rgb_imgs::print_square_at(50, colors::GREEN, position).unwrap();
    println!();

    let img_path = "/home/edcarney/wallpapers/pixel-night-city.png";
    let position = PositioningType::Centered;
    png_imgs::print_bounded_img_at(img_path, 100, 25, position).unwrap();
    println!();

    let position = PositioningType::Centered;
    rgb_imgs::print_square_at(50, colors::YELLOW, position).unwrap();
    println!();

    let img_path = "/home/edcarney/wallpapers/pixel-night-city.png";
    png_imgs::print_bounded_img(img_path, 50, 13).unwrap();
    println!();

    let img_path = "/home/edcarney/Downloads/red_arrow.png";
    png_imgs::print_img(img_path).unwrap();
    println!();

    let position = PositioningType::Centered;
    let img_path = "/home/edcarney/Downloads/red_arrow.png";
    png_imgs::print_img_at(img_path, position).unwrap();
    println!();
}

fn test_cursor_positioning() {
    let res_before_1 = CsiCommand::new("6n", "R").execute_with_response().unwrap();
    rgba_imgs::print_square(100, colors::GREEN.with_alpha(25)).unwrap();
    let res_after_1 = CsiCommand::new("6n", "R").execute_with_response().unwrap();

    let res_before_2 = CsiCommand::new("6n", "R").execute_with_response().unwrap();
    rgba_imgs::print_square(100, colors::GREEN.with_alpha(25)).unwrap();
    let res_after_2 = CsiCommand::new("6n", "R").execute_with_response().unwrap();
    println!();
    println!("Before: {res_before_1}, After: {res_after_1}");
    println!("Before: {res_before_2}, After: {res_after_2}");
}

fn test_printin() {
    let img_path = "/home/edcarney/wallpapers/pixel-night-city.png";

    rgba_imgs::print_square(50, colors::GREEN.with_alpha(25)).unwrap();
    rgba_imgs::print_square(50, colors::GREEN.with_alpha(50)).unwrap();
    rgba_imgs::print_square(50, colors::GREEN.with_alpha(100)).unwrap();
    rgba_imgs::print_square(50, colors::GREEN.with_alpha(200)).unwrap();
    rgba_imgs::print_square(50, colors::GREEN.with_alpha(u8::MAX)).unwrap();
    println!();

    rgb_imgs::print_square(200, colors::WHITE).unwrap();
    println!();

    png_imgs::print_bounded_img(img_path, 100, 25).unwrap();
    println!();
}

fn test_cmd_seqs() {
    let res = CsiCommand::new("6n", "R").execute_with_response().unwrap();
    println!("{res}");

    let res = CsiCommand::new("c", "c").execute_with_response().unwrap();
    println!("{res}");

    // let payload = vec![255, 255, 255];
    // let ctrl_data: Vec<String> = vec![
    //     Metadata::Id(32).get_ctrl_seq(),
    //     Transmission::Direct(&payload).get_ctrl_seq(),
    //     PixelFormat::Rgba {
    //         width: 1,
    //         height: 1,
    //     }
    //     .get_ctrl_seq(),
    //     Action::Query.get_ctrl_seq(),
    // ];
    // let res = KittyCommand::new(&payload, &ctrl_data)
    //     .execute_with_response()
    //     .unwrap();
    // println!("{res}");
}
