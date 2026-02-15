use libc::{TIOCGWINSZ, c_ushort};
use nix::ioctl_read_bad;
use std::{error::Error, fmt};

#[derive(Debug)]
pub enum WindowCtrlError {
    IoctlFailed { exit_code: i32 },
    InvalidDimensions { rows: u32, cols: u32 },
}

impl fmt::Display for WindowCtrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowCtrlError::IoctlFailed { exit_code } => {
                write!(f, "Non-zero exit code {} from ioctl read", exit_code)
            }
            WindowCtrlError::InvalidDimensions { rows, cols } => {
                write!(
                    f,
                    "Terminal reported invalid dimensions (rows={}, cols={}) — cannot render",
                    rows, cols
                )
            }
        }
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
    fn build_from_ioctl(ioctl_data: [u16; 4]) -> Result<WindowSize, WindowCtrlError> {
        let rows = ioctl_data[0] as u32;
        let cols = ioctl_data[1] as u32;
        let x_pix = ioctl_data[2] as u32;
        let y_pix = ioctl_data[3] as u32;

        if rows == 0 || cols == 0 || x_pix == 0 || y_pix == 0 {
            return Err(WindowCtrlError::InvalidDimensions { rows, cols });
        }

        let pix_per_col = x_pix / cols;
        let pix_per_row = y_pix / rows;
        Ok(WindowSize {
            rows,
            cols,
            x_pix,
            y_pix,
            pix_per_col,
            pix_per_row,
        })
    }
}

ioctl_read_bad!(get_window_size_unsafe, TIOCGWINSZ, c_ushort);

pub fn get_window_size() -> Result<WindowSize, Box<dyn Error>> {
    let mut data: [u16; 4] = [0, 0, 0, 0];
    let exit_code = unsafe { get_window_size_unsafe(0, data.as_mut_ptr())? };
    match exit_code {
        0 => Ok(WindowSize::build_from_ioctl(data)?),
        _ => Err(Box::new(WindowCtrlError::IoctlFailed { exit_code }) as Box<dyn Error>),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_from_ioctl_normal_values() {
        let data: [u16; 4] = [24, 80, 1920, 1080];
        let ws = WindowSize::build_from_ioctl(data).unwrap();
        assert_eq!(ws.rows, 24);
        assert_eq!(ws.cols, 80);
        assert_eq!(ws.x_pix, 1920);
        assert_eq!(ws.y_pix, 1080);
        assert_eq!(ws.pix_per_col, 24); // 1920 / 80
        assert_eq!(ws.pix_per_row, 45); // 1080 / 24
    }

    #[test]
    fn build_from_ioctl_zero_cols_returns_err() {
        let data: [u16; 4] = [24, 0, 1920, 1080];
        let result = WindowSize::build_from_ioctl(data);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("invalid dimensions"),
            "Error should mention invalid dimensions"
        );
    }

    #[test]
    fn build_from_ioctl_zero_rows_returns_err() {
        let data: [u16; 4] = [0, 80, 1920, 1080];
        let result = WindowSize::build_from_ioctl(data);
        assert!(result.is_err());
    }

    #[test]
    fn build_from_ioctl_all_zeros_returns_err() {
        let data: [u16; 4] = [0, 0, 0, 0];
        let result = WindowSize::build_from_ioctl(data);
        assert!(result.is_err());
    }

    #[test]
    fn build_from_ioctl_zero_pixels_returns_err() {
        // Nonzero rows/cols but zero pixel dimensions would produce
        // pix_per_col=0 and pix_per_row=0, causing downstream division-by-zero.
        let data: [u16; 4] = [24, 80, 0, 0];
        let result = WindowSize::build_from_ioctl(data);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("invalid dimensions"),
            "Error should mention invalid dimensions"
        );
    }

    #[test]
    fn build_from_ioctl_zero_x_pix_only_returns_err() {
        let data: [u16; 4] = [24, 80, 0, 1080];
        let result = WindowSize::build_from_ioctl(data);
        assert!(result.is_err());
    }

    #[test]
    fn build_from_ioctl_zero_y_pix_only_returns_err() {
        let data: [u16; 4] = [24, 80, 1920, 0];
        let result = WindowSize::build_from_ioctl(data);
        assert!(result.is_err());
    }
}
