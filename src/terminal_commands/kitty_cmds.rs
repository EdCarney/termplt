use super::responses::TermCommand;
use crate::kitty_graphics::ctrl_seq::{CtrlSeq, Metadata};
use crate::kitty_graphics::encoding;

const CMD_START: &[u8] = b"\x1B_G";
const CMD_SEP: &[u8] = b";";
const CMD_END: &[u8] = b"\x1B\\";
const MAX_PAYLOAD_SIZE: usize = 4096;

pub struct KittyCommand {
    cmd: Vec<u8>,
}

impl TermCommand for KittyCommand {
    fn req_start(&self) -> &[u8] {
        CMD_START
    }
    fn req_end(&self) -> &[u8] {
        CMD_END
    }
    fn res_start(&self) -> &[u8] {
        CMD_START
    }
    fn res_end(&self) -> &[u8] {
        CMD_END
    }
    fn get_request(&mut self) -> &[u8] {
        &self.cmd
    }
}

impl KittyCommand {
    pub fn new(payload: &[u8], ctrl_data: Vec<Box<dyn CtrlSeq>>) -> KittyCommand {
        let payload = encoding::read_bytes_to_b64(&payload).unwrap();
        let mut ctrl_data = Vec::from(ctrl_data);

        let chunks = payload.chunks(MAX_PAYLOAD_SIZE);
        let num_chunks = chunks.len();

        let mut cmd = Vec::<u8>::with_capacity(payload.len() + num_chunks * 10);
        for (ind, chunk) in chunks.enumerate() {
            let is_last = ind == num_chunks - 1;

            ctrl_data.push(Box::new(Metadata::MoreData(!is_last)));
            let ctrl_bytes = ctrl_data
                .drain(..)
                .map(|seq| seq.get_ctrl_seq())
                .collect::<Vec<_>>()
                .join(",")
                .into_bytes();

            cmd.extend_from_slice(CMD_START);
            cmd.extend_from_slice(&ctrl_bytes);
            cmd.extend_from_slice(CMD_SEP);
            cmd.extend_from_slice(&chunk);
            cmd.extend_from_slice(CMD_END);
        }

        KittyCommand { cmd }
    }
}
