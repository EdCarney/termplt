use libc::{TIOCGWINSZ, c_ushort};
use nix::ioctl_read_bad;
use std::{error::Error, fmt};

#[derive(Debug)]
pub struct WindowCtrlError {
    pub exit_code: i32,
}

impl fmt::Display for WindowCtrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Non-zero exit code {} from ioctl read", self.exit_code)
    }
}

impl Error for WindowCtrlError {}

#[derive(Debug)]
pub struct WindowSize {
    pub rows: u32,
    pub cols: u32,
    pub x_pix: u32,
    pub y_pix: u32,
    pub pix_per_row: u32,
    pub pix_per_col: u32,
}

impl WindowSize {
    fn build_from_ioctl(ioctl_data: [u16; 4]) -> WindowSize {
        let rows = ioctl_data[0] as u32;
        let cols = ioctl_data[1] as u32;
        let x_pix = ioctl_data[2] as u32;
        let y_pix = ioctl_data[3] as u32;
        let pix_per_col = x_pix / cols;
        let pix_per_row = y_pix / rows;
        WindowSize {
            rows,
            cols,
            x_pix,
            y_pix,
            pix_per_col,
            pix_per_row,
        }
    }
}

ioctl_read_bad!(get_window_size_unsafe, TIOCGWINSZ, c_ushort);

pub fn get_window_size() -> Result<WindowSize, Box<dyn Error>> {
    let mut data: [u16; 4] = [0, 0, 0, 0];
    let exit_code = unsafe { get_window_size_unsafe(0, data.as_mut_ptr())? };
    match exit_code {
        0 => Ok(WindowSize::build_from_ioctl(data)),
        _ => Err(Box::new(WindowCtrlError { exit_code }) as Box<dyn Error>),
    }
}
