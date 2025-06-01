use super::csi_cmds::CsiCommand;
use super::kitty_cmds::KittyCommand;
use crate::kitty_graphics::ctrl_seq::*;
use crossterm::terminal;
use std::{
    error::Error,
    fmt,
    io::{self, Read, Write},
    time::Instant,
};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub trait TermCommand {
    fn get_request(&self) -> &[u8];

    fn get_response_start(&self) -> &[u8] {
        "".as_bytes()
    }

    fn get_response_end(&self) -> &[u8] {
        "".as_bytes()
    }

    fn execute(&self) -> Result<()> {
        let mut stdout = io::stdout().lock();
        stdout.write_all(&self.get_request())?;
        stdout.flush()?;
        Ok(())
    }

    fn execute_with_response(&self) -> Result<String> {
        let resp_start = self.get_response_start();
        let resp_end = self.get_response_end();

        let mut buf = Vec::<u8>::new();
        let mut stdin = std::io::stdin().lock();
        let mut byte_buf = [0u8; 1];
        let mut resp_recvd = false;

        terminal::enable_raw_mode()?;
        self.execute()?;

        let watch = Instant::now();
        while watch.elapsed().as_millis() < 1000 {
            match stdin.read_exact(&mut byte_buf) {
                Ok(_) => {
                    buf.push(byte_buf[0]);
                    if buf.len() > resp_start.len() && buf.ends_with(resp_end) {
                        if buf.starts_with(resp_start) {
                            resp_recvd = true;
                            break;
                        }

                        // if buffer ends with the correct bytes but does not start with the correct
                        // bytes, then it is not what we are looking for; clear and start again
                        buf.clear();
                    }
                }
                _ => (),
            }
        }

        terminal::disable_raw_mode()?;

        if resp_recvd {
            let start = resp_start.len();
            let end = buf.len() - resp_end.len();
            let resp = String::from_utf8(buf[start..end].to_vec())?;
            Ok(resp)
        } else {
            Err(Box::new(TerminalCommandError {
                // TODO: fix error construction from cmd()
                failed_cmd: "".to_string(), // cmd.cmd().unwrap(),
            }))
        }
    }
}

#[derive(Debug)]
pub struct TerminalCommandError {
    failed_cmd: String,
}

impl fmt::Display for TerminalCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error executing terminal command: {}", self.failed_cmd)
    }
}

impl Error for TerminalCommandError {}

// pub fn read_command() -> Result<()> {
//     let mut cmd = CsiCommand::new("6n", "R");
//     let resp = execute_and_read(&mut cmd)?;
//     println!("CSI Resp: {} ({resp:?})", resp_to_str(&resp, &cmd)?);
//
//     let mut cmd = CsiCommand::new("c", "c");
//     let resp = execute_and_read(&mut cmd)?;
//     println!("CSI Resp: {} ({resp:?})", resp_to_str(&resp, &cmd)?);
//
//     let payload: Vec<u8> = vec![255, 255, 255];
//     let ctrl_data: Vec<Box<dyn CtrlSeq>> = vec![
//         Box::new(Metadata::Id(32)),
//         Box::new(Transmission::Direct),
//         Box::new(PixelFormat::Rgb {
//             width: 1,
//             height: 1,
//         }),
//         Box::new(Action::Query),
//     ];
//     let mut cmd_kitty = KittyCommand::new(&payload, ctrl_data);
//     let resp = execute_and_read(&mut cmd_kitty)?;
//     println!("Kitty Resp: {} ({resp:?})", resp_to_str(&resp, &cmd_kitty)?);
//
//     Ok(())
// }
