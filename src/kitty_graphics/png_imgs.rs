use std::collections::HashMap;
use std::fs;

use super::{encoding, term_ctrl};

pub fn print_img(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    let img_data = encoding::read_bytes_to_b64(&bytes)?;
    let ctrl_data = HashMap::from([("a", "T"), ("f", "100")]);
    term_ctrl::write_img_data(&img_data, &ctrl_data)
}

pub fn print_bounded_img(
    path: &str,
    width: u32,
    height: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    let img_data = encoding::read_bytes_to_b64(&bytes)?;
    let c_str = format!("{width}");
    let r_str = format!("{height}");
    let ctrl_data = HashMap::from([("a", "T"), ("f", "100"), ("c", &c_str), ("r", &r_str)]);
    term_ctrl::write_img_data(&img_data, &ctrl_data)
}
