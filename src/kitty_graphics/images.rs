use super::ctrl_seq::PixelFormat;
use crate::terminal_commands::csi_cmds;

pub struct Image {
    format: PixelFormat,
    x_pos_pix: u32,
    y_pos_pix: u32,
    data: Vec<u8>,
}

impl Image {
    pub fn at_current_position(format: PixelFormat, data: &[u8]) -> Image {
        let cursor_pos = csi_cmds::get_cursor_pos().expect("Failure getting cursor position");
        let window_sz = termplt::get_window_size().expect("Failure getting terminal window size");

        let x_pos_pix = ((cursor_pos.col - 1) as f32 * window_sz.pix_per_col) as u32;
        let y_pos_pix = ((cursor_pos.row - 1) as f32 * window_sz.pix_per_row) as u32;

        println!("Creating img at current position...");
        println!("\t- cursor pos: {cursor_pos:?}");
        println!("\t- window size: {window_sz:?}");
        println!("\t- position in pix: x={x_pos_pix}, y={y_pos_pix}");

        Image {
            format,
            x_pos_pix,
            y_pos_pix,
            data: data.to_vec(),
        }
    }
}

