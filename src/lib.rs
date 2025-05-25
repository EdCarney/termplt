mod window_ctrl;

pub use window_ctrl::get_window_size;

pub fn print_hello() {
    let winsz = window_ctrl::get_window_size();
    println!("{winsz:?}");
}
