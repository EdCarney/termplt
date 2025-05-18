use nix_ioctl::get_window_size;

fn main() {
    let window_size = get_window_size();

    println!("{window_size:?}");
}
