use std::collections::HashMap;

use super::{encoding, term_ctrl};

pub fn print_red_square(size: usize) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Vec::with_capacity(size * size * 3);
    for _ in 0..(size * size) {
        let mut red: Vec<u8> = vec![255, 0, 0];
        bytes.append(&mut red);
    }
    let img_data = encoding::read_bytes_to_b64(&bytes)?;
    let size_str = format!("{size}");
    let ctrl_data = HashMap::from([("a", "T"), ("f", "24"), ("s", &size_str), ("v", &size_str)]);
    term_ctrl::write_img_data(&img_data, &ctrl_data)
}
