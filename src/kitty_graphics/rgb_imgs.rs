use rgb::RGB8;

use super::{
    ctrl_seq::{Action, CtrlSeq, PixelFormat},
    encoding, term_ctrl,
};

pub fn print_square(size: usize, color: RGB8) -> Result<(), Box<dyn std::error::Error>> {
    let bytes: Vec<u8> = (0..(size * size))
        .flat_map(|_| vec![color.r, color.g, color.b])
        .collect();

    let img_data = encoding::read_bytes_to_b64(&bytes)?;

    let width = size as u32;
    let height = size as u32;

    let ctrl_data: Vec<Box<dyn CtrlSeq>> = vec![
        Box::new(Action::TransmitDisplay),
        Box::new(PixelFormat::Rgb { width, height }),
    ];

    term_ctrl::write_img_data(&img_data, ctrl_data)
}
