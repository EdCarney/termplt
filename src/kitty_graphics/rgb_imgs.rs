use super::{
    ctrl_seq::{Action, CtrlSeq, PixelFormat},
    encoding, term_ctrl,
};

pub fn print_red_square(size: usize) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Vec::with_capacity(size * size * 3);
    for _ in 0..(size * size) {
        let mut red: Vec<u8> = vec![255, 0, 0];
        bytes.append(&mut red);
    }
    let img_data = encoding::read_bytes_to_b64(&bytes)?;

    let width = size as u32;
    let height = size as u32;

    let ctrl_data: Vec<Box<dyn CtrlSeq>> = vec![
        Box::new(Action::TransmitDisplay),
        Box::new(PixelFormat::Rgb { width, height }),
    ];

    term_ctrl::write_img_data(&img_data, &ctrl_data)
}
