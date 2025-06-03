use super::ctrl_seq::{PixelFormat, Transmission};
use crate::terminal_commands::images::{Image, PositioningType};
use rgb::RGB8;

pub fn print_square(size: usize, color: RGB8) -> Result<(), Box<dyn std::error::Error>> {
    let bytes: Vec<u8> = (0..(size * size))
        .flat_map(|_| vec![color.r, color.g, color.b])
        .collect();

    let width = size as u32;
    let height = size as u32;
    let format = PixelFormat::Rgb { width, height };
    let transmission = Transmission::Direct(bytes);
    Image::new(format, transmission)?.display()
}

pub fn print_square_at(
    size: usize,
    color: RGB8,
    positioning_type: PositioningType,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes: Vec<u8> = (0..(size * size))
        .flat_map(|_| vec![color.r, color.g, color.b])
        .collect();

    let width = size as u32;
    let height = size as u32;
    let format = PixelFormat::Rgb { width, height };
    let transmission = Transmission::Direct(bytes);
    Image::new(format, transmission)?.display_at_position(positioning_type)
}
