use crate::kitty_graphics::ctrl_seq::*;

use kitty_graphics::images::Image;
use rgb::RGB8;
use terminal_commands::{csi_cmds::CsiCommand, kitty_cmds::KittyCommand, responses::TermCommand};

mod common;
mod kitty_graphics;
mod terminal_commands;

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
    let data = vec![];
    let format = PixelFormat::Rgb {
        width: 10,
        height: 10,
    };
    let img = Image::at_current_position(format, &data);
}

fn test_cursor_positioning() {
    let res_before_1 = CsiCommand::new("6n", "R").execute_with_response().unwrap();
    kitty_graphics::rgba_imgs::print_square(100, GREEN.with_alpha(25)).unwrap();
    let res_after_1 = CsiCommand::new("6n", "R").execute_with_response().unwrap();

    let res_before_2 = CsiCommand::new("6n", "R").execute_with_response().unwrap();
    kitty_graphics::rgba_imgs::print_square(100, GREEN.with_alpha(25)).unwrap();
    let res_after_2 = CsiCommand::new("6n", "R").execute_with_response().unwrap();
    println!();
    println!("Before: {res_before_1}, After: {res_after_1}");
    println!("Before: {res_before_2}, After: {res_after_2}");
}

fn test_printin() {
    let img_path = "/home/edcarney/wallpapers/pixel-night-city.png";

    kitty_graphics::rgba_imgs::print_square(50, GREEN.with_alpha(25)).unwrap();
    kitty_graphics::rgba_imgs::print_square(50, GREEN.with_alpha(50)).unwrap();
    kitty_graphics::rgba_imgs::print_square(50, GREEN.with_alpha(100)).unwrap();
    kitty_graphics::rgba_imgs::print_square(50, GREEN.with_alpha(200)).unwrap();
    kitty_graphics::rgba_imgs::print_square(50, GREEN.with_alpha(u8::MAX)).unwrap();
    println!();

    kitty_graphics::rgb_imgs::print_square(200, WHITE).unwrap();
    println!();

    // kitty_graphics::png_imgs::print_img(img_path).unwrap();
    // println!();

    kitty_graphics::png_imgs::print_bounded_img(img_path, 100, 25).unwrap();
    println!();
}

fn test_cmd_seqs() {
    let res = CsiCommand::new("6n", "R").execute_with_response().unwrap();
    println!("{res}");

    let res = CsiCommand::new("c", "c").execute_with_response().unwrap();
    println!("{res}");

    let payload: Vec<u8> = vec![255, 255, 255];
    let ctrl_data: Vec<Box<dyn CtrlSeq>> = vec![
        Box::new(Metadata::Id(32)),
        Box::new(Transmission::Direct),
        Box::new(PixelFormat::Rgba {
            width: 1,
            height: 1,
        }),
        Box::new(Action::Query),
    ];
    let res = KittyCommand::new(&payload, ctrl_data)
        .execute_with_response()
        .unwrap();
    println!("{res}");
}
