use std::f32;

use rgb::RGB8;
use termplt::{
    kitty_graphics::{
        ctrl_seq::{PixelFormat, Transmission},
        png_imgs, rgb_imgs, rgba_imgs,
    },
    plotting::{
        canvas::{BufferType, TerminalCanvas},
        colors,
        graph::Graph,
        marker::MarkerStyle,
        point::Point,
        series::Series,
    },
    terminal_commands::{
        csi_cmds::{self, CsiCommand},
        images::{Image, PositioningType},
        responses::TermCommand,
    },
};

const RED: RGB8 = RGB8 { r: 255, g: 0, b: 0 };
const GREEN: RGB8 = RGB8 { r: 0, g: 255, b: 0 };
const BLUE: RGB8 = RGB8 { r: 0, g: 0, b: 255 };
const YELLOW: RGB8 = RGB8 {
    r: 255,
    g: 255,
    b: 0,
};
const WHITE: RGB8 = RGB8 {
    r: 255,
    g: 255,
    b: 255,
};
const BLACK: RGB8 = RGB8 { r: 0, g: 0, b: 0 };

fn main() {
    let width = 300;
    let height = 300;

    let points = (0..=20)
        .map(|x| Point::new(x, x * x))
        .collect::<Vec<Point<_>>>();

    let bytes_1 = TerminalCanvas::new(width + 100, height + 100, colors::BLACK)
        .with_buffer(BufferType::Uniform(15))
        .with_graph(
            Graph::new().with_series(Series::new(&points).with_marker_style(
                MarkerStyle::HollowSquare {
                    size: 5,
                    color: colors::WHITE,
                },
            )),
        )
        .draw()
        .unwrap()
        .get_bytes();

    let num_points = 100;
    let points = (0..num_points)
        .map(|x| {
            let x = (x as f32) * (2. * f32::consts::PI / (num_points as f32));
            Point::new(x, x.sin())
        })
        .collect::<Vec<Point<_>>>();
    let bytes_2 = TerminalCanvas::new(width, height, colors::BLACK)
        .with_buffer(BufferType::Uniform(15))
        .with_graph(Graph::new().with_series(Series::new(&points)))
        .draw()
        .unwrap()
        .get_bytes();

    let format = PixelFormat::Rgb {
        width: width + 100,
        height: height + 100,
    };
    let transmission = Transmission::Direct(bytes_1);
    Image::new(format, transmission).unwrap().display().unwrap();
    println!();

    let format = PixelFormat::Rgb { width, height };
    let transmission = Transmission::Direct(bytes_2);
    Image::new(format, transmission).unwrap().display().unwrap();
    println!();
}

fn test_new_terminal_cmds() {
    csi_cmds::clear_screen().unwrap();

    rgb_imgs::print_square(50, RED).unwrap();
    println!();

    let position = PositioningType::ExactPixel { x: 10, y: 10 };
    rgb_imgs::print_square_at(50, GREEN, position).unwrap();
    println!();

    let img_path = "/home/edcarney/wallpapers/pixel-night-city.png";
    let position = PositioningType::Centered;
    png_imgs::print_bounded_img_at(img_path, 100, 25, position).unwrap();
    println!();

    let position = PositioningType::Centered;
    rgb_imgs::print_square_at(50, YELLOW, position).unwrap();
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
    rgba_imgs::print_square(100, GREEN.with_alpha(25)).unwrap();
    let res_after_1 = CsiCommand::new("6n", "R").execute_with_response().unwrap();

    let res_before_2 = CsiCommand::new("6n", "R").execute_with_response().unwrap();
    rgba_imgs::print_square(100, GREEN.with_alpha(25)).unwrap();
    let res_after_2 = CsiCommand::new("6n", "R").execute_with_response().unwrap();
    println!();
    println!("Before: {res_before_1}, After: {res_after_1}");
    println!("Before: {res_before_2}, After: {res_after_2}");
}

fn test_printin() {
    let img_path = "/home/edcarney/wallpapers/pixel-night-city.png";

    rgba_imgs::print_square(50, GREEN.with_alpha(25)).unwrap();
    rgba_imgs::print_square(50, GREEN.with_alpha(50)).unwrap();
    rgba_imgs::print_square(50, GREEN.with_alpha(100)).unwrap();
    rgba_imgs::print_square(50, GREEN.with_alpha(200)).unwrap();
    rgba_imgs::print_square(50, GREEN.with_alpha(u8::MAX)).unwrap();
    println!();

    rgb_imgs::print_square(200, WHITE).unwrap();
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
