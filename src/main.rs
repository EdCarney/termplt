mod kitty_graphics;

fn main() {
    let window_size = termplt::get_window_size();
    println!("{window_size:?}");

    kitty_graphics::rgb_imgs::print_red_square(100).unwrap();

    kitty_graphics::png_imgs::print_bounded_img(
        "/home/edcarney/wallpapers/pixel-night-city.png",
        100,
        25,
    )
    .unwrap();
}
