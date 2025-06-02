use super::ctrl_seq::{PixelFormat, Transmission};
use crate::terminal_commands::images::{Image, PositioningType};

pub fn print_img(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let window_sz = termplt::get_window_size()?;
    let rows = window_sz.rows as u32;
    let cols = window_sz.cols as u32;
    let format = PixelFormat::PngBounded { rows, cols };
    let transmission = Transmission::File(String::from(path));
    Image::new(format, transmission).display()
}

pub fn print_bounded_img(
    path: &str,
    cols: u32,
    rows: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = PixelFormat::PngBounded { rows, cols };
    let transmission = Transmission::File(String::from(path));
    Image::new(format, transmission).display()
}

pub fn print_bounded_img_at(
    path: &str,
    cols: u32,
    rows: u32,
    position_type: PositioningType,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = PixelFormat::PngBounded { rows, cols };
    let transmission = Transmission::File(String::from(path));
    Image::new(format, transmission).display_at_position(position_type)
}
