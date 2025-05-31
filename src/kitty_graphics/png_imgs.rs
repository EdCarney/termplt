use super::ctrl_seq::{Action, CtrlSeq, PixelFormat};
use crate::terminal_commands::{kitty_cmds::KittyCommand, responses::execute};
use std::fs;

pub fn print_img(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    let window_sz = termplt::get_window_size()?;

    let rows = window_sz.rows as u32;
    let cols = window_sz.cols as u32;

    let ctrl_data: Vec<Box<dyn CtrlSeq>> = vec![
        Box::new(Action::TransmitDisplay),
        Box::new(PixelFormat::PngBounded { cols, rows }),
    ];

    let mut cmd = KittyCommand::new(&bytes, ctrl_data);
    execute(&mut cmd)?;
    Ok(())
}

pub fn print_bounded_img(
    path: &str,
    cols: u32,
    rows: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    let ctrl_data: Vec<Box<dyn CtrlSeq>> = vec![
        Box::new(Action::TransmitDisplay),
        Box::new(PixelFormat::PngBounded { cols, rows }),
    ];

    let mut cmd = KittyCommand::new(&bytes, ctrl_data);
    execute(&mut cmd)?;
    Ok(())
}
