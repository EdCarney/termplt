use super::ctrl_seq::{CtrlSeq, Metadata};
use std::io::{self, Write};

const START: &[u8] = b"\x1B_G";
const SEP: &[u8] = b";";
const END: &[u8] = b"\x1B\\";

pub fn write_img_data(
    img_data: &[u8],
    mut ctrl_data: Vec<Box<dyn CtrlSeq>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let chunks = img_data.chunks(4096);
    let num_chunks = chunks.len();
    let mut handle = io::stdout().lock();

    for (ind, chunk) in chunks.enumerate() {
        let last_chunk = ind == num_chunks - 1;
        ctrl_data.push(Box::new(Metadata::MoreData(!last_chunk)));

        let ctl_bytes = ctrl_data
            .drain(..)
            .map(|seq| seq.get_ctrl_seq())
            .collect::<Vec<_>>()
            .join(",")
            .into_bytes();

        handle.write_all(START)?;
        handle.write_all(&ctl_bytes)?;
        handle.write_all(SEP)?;
        handle.write_all(chunk)?;
        handle.write_all(END)?;
    }
    handle.flush()?;
    Ok(())
}
