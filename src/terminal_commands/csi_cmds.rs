use super::responses::TermCommand;

const CMD_START: &[u8] = b"\x1b[";

pub struct CsiCommand {
    cmd: Vec<u8>,
    cmd_end: Vec<u8>,
}

impl CsiCommand {
    pub fn new(command: &str, command_end: &str) -> CsiCommand {
        let mut cmd = Vec::from(CMD_START);
        cmd.extend_from_slice(command.as_bytes());

        CsiCommand {
            cmd,
            cmd_end: command_end.as_bytes().to_vec(),
        }
    }
}

impl TermCommand for CsiCommand {
    fn get_request(&self) -> &[u8] {
        &self.cmd
    }

    fn get_response_start(&self) -> &[u8] {
        CMD_START
    }

    fn get_response_end(&self) -> &[u8] {
        &self.cmd_end
    }
}
