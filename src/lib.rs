use libc::{TIOCGWINSZ, c_ushort};
use nix::ioctl_read_bad;

#[derive(Debug)]
pub struct WindowSize {
    rows: u16,
    cols: u16,
    x_pixels: u16,
    y_pixels: u16,
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
    let data = unsafe {
        let mut data: [u16; 4] = [0, 0, 0, 0];
        let data_ptr = data.as_mut_ptr();
        get_window_size_unsafe(0, data_ptr).unwrap();
        data
    };
    WindowSize::build_from_ioctl(data)
}
