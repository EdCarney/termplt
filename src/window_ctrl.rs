use libc::{TIOCGWINSZ, c_ushort};
use nix::ioctl_read_bad;

#[derive(Debug)]
pub struct WindowSize {
    pub rows: u16,
    pub cols: u16,
    pub x_pixels: u16,
    pub y_pixels: u16,
}

impl WindowSize {
    fn build_from_ioctl(ioctl_data: [u16; 4]) -> WindowSize {
        WindowSize {
            rows: ioctl_data[0],
            cols: ioctl_data[1],
            x_pixels: ioctl_data[2],
            y_pixels: ioctl_data[3],
        }
    }
}

ioctl_read_bad!(get_window_size_unsafe, TIOCGWINSZ, c_ushort);

pub fn get_window_size() -> WindowSize {
    let mut data: [u16; 4] = [0, 0, 0, 0];
    unsafe {
        get_window_size_unsafe(0, data.as_mut_ptr()).unwrap();
    }
    WindowSize::build_from_ioctl(data)
}
