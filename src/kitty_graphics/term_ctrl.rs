use super::ctrl_seq::{CtrlSeq, Metadata};
use std::io::{self, Write};

const START: &[u8] = b"\x1B";
const SEP: &[u8] = b";";
const END: &[u8] = b"\x1B\\";

pub fn query_term_capabilities() {}

pub fn query_device_capabilities() {}

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

        let mut start = START.to_vec();
        let mut control_data = ctl_bytes.to_vec();
        let mut sep = SEP.to_vec();
        let mut payload = chunk.to_vec();
        let mut term = END.to_vec();

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
