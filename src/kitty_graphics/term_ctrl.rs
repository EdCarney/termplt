use std::io::{self, Write};

use crate::kitty_graphics::ctrl_seq::Metadata;

use super::ctrl_seq::CtrlSeq;

pub fn write_img_data(
    img_data: &[u8],
    mut ctrl_data: Vec<Box<dyn CtrlSeq>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let chunks = img_data.chunks(4096);
    let num_chunks = chunks.len();

    let mut cmd: Vec<u8> = Vec::with_capacity(4200);
    let mut handle = io::stdout().lock();

    for (ind, chunk) in chunks.enumerate() {
        ctrl_data.push(Box::new(Metadata::MoreData(ind < num_chunks - 1)));

        let ctl_bytes = ctrl_data
            .drain(..)
            .map(|seq| seq.get_ctrl_seq())
            .collect::<Vec<_>>()
            .join(",")
            .into_bytes();

        let mut start = b"\x1B_G".to_vec();
        let mut control_data = ctl_bytes.to_vec();
        let mut sep = b";".to_vec();
        let mut payload = chunk.to_vec();
        let mut term = b"\x1B\\".to_vec();

        cmd.append(&mut start);
        cmd.append(&mut control_data);
        cmd.append(&mut sep);
        cmd.append(&mut payload);
        cmd.append(&mut term);

        handle.write_all(cmd.drain(..).as_slice())?;
        handle.flush()?;
    }
    handle.write(b"\n")?;

    Ok(())
}
