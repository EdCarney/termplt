use std::io::{self, Write};

use super::ctrl_seq::CtrlSeq;

pub fn write_img_data(
    img_data: &[u8],
    ctrl_data: &[Box<dyn CtrlSeq>],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut handle = io::stdout().lock();
    let chunks = img_data.chunks(4096);
    let num_chunks = chunks.len();
    let mut cmd: Vec<u8> = vec![];

    for (ind, chunk) in chunks.enumerate() {
        let mut control_data = if ind == 0 {
            ctrl_data.iter().fold(String::new(), |acc, seq| {
                format!("{},{acc}", seq.get_ctrl_seq())
            })
        } else {
            String::new()
        };

        control_data += if ind == num_chunks - 1 { "m=0" } else { "m=1" };

        cmd.clear();
        let mut start = b"\x1B_G".to_vec();
        let mut control_data = control_data.as_bytes().to_vec();
        let mut sep = b";".to_vec();
        let mut payload = chunk.to_vec();
        let mut term = b"\x1B\\".to_vec();

        cmd.append(&mut start);
        cmd.append(&mut control_data);
        cmd.append(&mut sep);
        cmd.append(&mut payload);
        cmd.append(&mut term);

        handle.write_all(cmd.as_slice())?;
        handle.flush()?;
    }
    handle.write(b"\n")?;

    Ok(())
}
