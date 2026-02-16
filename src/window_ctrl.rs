use crate::terminal_commands::csi_cmds;
use std::{error::Error, fmt};

#[derive(Debug)]
pub enum WindowCtrlError {
    InvalidDimensions { rows: u32, cols: u32 },
}

impl fmt::Display for WindowCtrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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

pub fn get_window_size() -> Result<WindowSize, Box<dyn Error>> {
    let (x_pix, y_pix) = csi_cmds::get_text_area_size_pixels()?;
    let (rows, cols) = csi_cmds::get_text_area_size_cells()?;

    if rows == 0 || cols == 0 || x_pix == 0 || y_pix == 0 {
        return Err(Box::new(WindowCtrlError::InvalidDimensions { rows, cols }));
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
