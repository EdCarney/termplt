use rgb::RGB8;

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
    terminal_commands::responses::read_command().unwrap();
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
