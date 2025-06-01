use super::ctrl_seq::{Action, CtrlSeq, PixelFormat};
use crate::terminal_commands::{kitty_cmds::KittyCommand, responses::TermCommand};
use rgb::RGBA8;

pub fn print_square(size: usize, color: RGBA8) -> Result<(), Box<dyn std::error::Error>> {
    let bytes: Vec<u8> = (0..(size * size))
        .flat_map(|_| vec![color.r, color.g, color.b, color.a])
        .collect();

    let width = size as u32;
    let height = size as u32;

    let ctrl_data: Vec<Box<dyn CtrlSeq>> = vec![
        Box::new(Action::TransmitDisplay),
        Box::new(PixelFormat::Rgba { width, height }),
    ];

    KittyCommand::new(&bytes, ctrl_data).execute()
}
