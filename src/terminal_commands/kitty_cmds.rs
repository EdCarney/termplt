use super::responses::TermCommand;
use crate::kitty_graphics::ctrl_seq::{CtrlSeq, Metadata};
use crate::kitty_graphics::encoding;

const CMD_START: &[u8] = b"\x1b_G";
const CMD_END: &[u8] = b"\x1b\\";
const CMD_SEP: &[u8] = b";";
const MAX_PAYLOAD_SIZE: usize = 4096;

pub struct KittyCommand {
    cmds: Vec<Vec<u8>>,
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
    fn generate_next_request(&mut self) -> Option<Vec<u8>> {
        self.cmds.pop()
    }
}

impl KittyCommand {
    pub fn new(payload: &[u8], ctrl_data: Vec<Box<dyn CtrlSeq>>) -> KittyCommand {
        let num_chunks = payload.len() / MAX_PAYLOAD_SIZE
            + if payload.len() % MAX_PAYLOAD_SIZE > 0 {
                1
            } else {
                0
            };

        let payload = encoding::read_bytes_to_b64(&payload).unwrap();
        let mut ctrl_data = Vec::from(ctrl_data);

        let cmds: Vec<Vec<u8>> = payload
            .chunks(MAX_PAYLOAD_SIZE)
            .enumerate()
            .map(|(ind, chunk)| {
                let is_last = ind == num_chunks - 1;

                ctrl_data.push(Box::new(Metadata::MoreData(!is_last)));
                let ctrl_str = ctrl_data
                    .drain(..)
                    .map(|seq| seq.get_ctrl_seq())
                    .collect::<Vec<_>>()
                    .join(",")
                    .into_bytes();

                let mut cmd = Vec::new();

                cmd.extend_from_slice(CMD_START);
                cmd.extend_from_slice(&ctrl_str);
                cmd.extend_from_slice(CMD_SEP);
                cmd.extend_from_slice(&chunk);
                cmd.extend_from_slice(CMD_END);

                cmd
            })
            .rev()
            .collect();

        KittyCommand { cmds }
    }
}
