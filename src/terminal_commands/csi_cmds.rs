use super::responses::TermCommand;

const CSI_START: &[u8] = b"\x1b[";

pub struct CsiCommand {
    cmd: Vec<u8>,
    res_end: Vec<u8>,
}

impl CsiCommand {
    pub fn new(cmd: &str, response_end: &str) -> CsiCommand {
        CsiCommand {
            cmd: Vec::from(cmd.as_bytes()),
            res_end: Vec::from(response_end.as_bytes()),
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
    fn generate_next_request(&mut self) -> Option<Vec<u8>> {
        if self.cmd.is_empty() {
            None
        } else {
            let mut req = Vec::new();
            req.extend_from_slice(self.req_start());
            req.extend_from_slice(self.cmd.drain(..).as_slice());
            req.extend_from_slice(self.req_end());
            Some(req)
        }
    }
}
