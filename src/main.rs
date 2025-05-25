use std::collections::HashMap;

mod kitty_graphics;

fn main() {
    let window_size = termplt::get_window_size();
    println!("{window_size:?}");

    let bytes = [1, 2, 3, 4, 5];
    let mut ctrl_data = HashMap::new();
    ctrl_data.insert("a", "T");
    ctrl_data.insert("t", "d");
    kitty_graphics::term_ctrl::write_img_data(&bytes, &ctrl_data).unwrap();
}
