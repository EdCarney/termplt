use super::ctrl_seq::{PixelFormat, Transmission};
use crate::terminal_commands::images::{Image, PositioningType};

pub fn print_img(path: &str) -> crate::Result<()> {
    let format = PixelFormat::Png;
    let transmission = Transmission::File(String::from(path));
    Image::new(format, transmission)?.display()
}

pub fn print_img_at(path: &str, position_type: PositioningType) -> crate::Result<()> {
    let format = PixelFormat::Png;
    let transmission = Transmission::File(String::from(path));
    Image::new(format, transmission)?.display_at_position(position_type)
}

pub fn print_bounded_img(path: &str, cols: u32, rows: u32) -> crate::Result<()> {
    let format = PixelFormat::PngBounded { rows, cols };
    let transmission = Transmission::File(String::from(path));
    Image::new(format, transmission)?.display()
}

pub fn print_bounded_img_at(
    path: &str,
    cols: u32,
    rows: u32,
    position_type: PositioningType,
) -> crate::Result<()> {
    let format = PixelFormat::PngBounded { rows, cols };
    let transmission = Transmission::File(String::from(path));
    Image::new(format, transmission)?.display_at_position(position_type)
}
