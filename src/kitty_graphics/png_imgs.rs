use std::fs;

use super::{
    ctrl_seq::{Action, CtrlSeq, PixelFormat},
    encoding, term_ctrl,
};

pub fn print_img(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    let img_data = encoding::read_bytes_to_b64(&bytes)?;
    let ctrl_data: Vec<Box<dyn CtrlSeq>> = vec![
        Box::new(Action::TransmitDisplay),
        Box::new(PixelFormat::Png),
    ];
    term_ctrl::write_img_data(&img_data, &ctrl_data)
}

pub fn print_bounded_img(
    path: &str,
    cols: u32,
    rows: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    let img_data = encoding::read_bytes_to_b64(&bytes)?;
    let ctrl_data: Vec<Box<dyn CtrlSeq>> = vec![
        Box::new(Action::TransmitDisplay),
        Box::new(PixelFormat::PngBounded { cols, rows }),
    ];

    term_ctrl::write_img_data(&img_data, &ctrl_data)
}
