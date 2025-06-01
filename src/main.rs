use crate::kitty_graphics::ctrl_seq::*;

use rgb::RGB8;
use terminal_commands::{csi_cmds::CsiCommand, kitty_cmds::KittyCommand, responses::TermCommand};

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
    test_printin();

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
