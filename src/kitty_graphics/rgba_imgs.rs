use super::ctrl_seq::{PixelFormat, Transmission};
use crate::terminal_commands::images::Image;
use rgb::RGBA8;

pub fn print_square(size: usize, color: RGBA8) -> Result<(), Box<dyn std::error::Error>> {
    let bytes: Vec<u8> = (0..(size * size))
        .flat_map(|_| vec![color.r, color.g, color.b, color.a])
        .collect();

    let width = size as u32;
    let height = size as u32;
    let format = PixelFormat::Rgba { width, height };
    let transmission = Transmission::Direct(bytes);
    Image::new(format, transmission).display()
}
