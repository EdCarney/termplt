use super::responses::TermCommand;

const CSI_START: &[u8] = b"\x1b[";

pub struct CsiCommand {
    cmd: Vec<u8>,
    res_end: Vec<u8>,
}

impl CsiCommand {
    pub fn new(command: &str, res_end: &str) -> CsiCommand {
        let mut cmd = Vec::new();
        cmd.extend_from_slice(CSI_START);
        cmd.extend_from_slice(command.as_bytes());
        cmd.extend_from_slice(res_end.as_bytes());

        CsiCommand {
            cmd,
            res_end: res_end.as_bytes().to_vec(),
        }
    }
}

impl TermCommand for CsiCommand {
    fn req_start(&self) -> &[u8] {
        CSI_START
    }
    fn res_start(&self) -> &[u8] {
        CSI_START
    }
    fn res_end(&self) -> &[u8] {
        &self.res_end
    }
    fn get_request(&mut self) -> &[u8] {
        &self.cmd
    }
}
