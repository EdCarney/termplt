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
    fn req_start(&self) -> &[u8] {
        "".as_bytes()
    }
    fn req_end(&self) -> &[u8] {
        "".as_bytes()
    }
    fn res_start(&self) -> &[u8] {
        "".as_bytes()
    }
    fn res_end(&self) -> &[u8] {
        "".as_bytes()
    }
    fn get_request(&mut self) -> &[u8];
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

pub fn execute<T: TermCommand>(cmd: &mut T) -> Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(&cmd.get_request())?;
    stdout.flush()?;
    Ok(())
}

fn execute_and_read<T: TermCommand>(cmd: &mut T) -> Result<Vec<u8>> {
    terminal::enable_raw_mode()?;

    execute(cmd)?;

    let res_start = cmd.res_start();
    let res_end = cmd.res_end();

    let mut buf = Vec::<u8>::new();
    let mut stdin = std::io::stdin().lock();
    let mut byte_buf = [0u8; 1];

    let mut response_recvd = false;
    let watch = Instant::now();
    while watch.elapsed().as_millis() < 1000 {
        match stdin.read_exact(&mut byte_buf) {
            Ok(_) => {
                buf.push(byte_buf[0]);
                if buf.len() > res_start.len() && buf.ends_with(&res_end) {
                    if buf.starts_with(&res_start) {
                        response_recvd = true;
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

    if response_recvd {
        Ok(buf)
    } else {
        Err(Box::new(TerminalCommandError {
            // TODO: fix error construction from cmd()
            failed_cmd: "".to_string(), // cmd.cmd().unwrap(),
        }))
    }
}

fn resp_to_str<T: TermCommand>(resp: &[u8], cmd: &T) -> Result<String> {
    if resp.starts_with(cmd.res_start()) && resp.ends_with(cmd.res_end()) {
        let start = cmd.res_start().len();
        let end = resp.len() - cmd.res_end().len();
        Ok(String::from_utf8(resp[start..end].to_vec())?)
    } else {
        Err(Box::new(TerminalCommandError {
            // TODO: fix error construction from cmd()
            failed_cmd: "".to_string(), // cmd.cmd().unwrap(),
        }))
    }
}

pub fn read_command() -> Result<()> {
    let mut cmd = CsiCommand::new("6n", "R");
    let resp = execute_and_read(&mut cmd)?;
    println!("CSI Resp: {} ({resp:?})", resp_to_str(&resp, &cmd)?);

    let mut cmd = CsiCommand::new("c", "c");
    let resp = execute_and_read(&mut cmd)?;
    println!("CSI Resp: {} ({resp:?})", resp_to_str(&resp, &cmd)?);

    let payload: Vec<u8> = vec![255, 255, 255];
    let ctrl_data: Vec<Box<dyn CtrlSeq>> = vec![
        Box::new(Metadata::Id(32)),
        Box::new(Transmission::Direct),
        Box::new(PixelFormat::Rgb {
            width: 1,
            height: 1,
        }),
        Box::new(Action::Query),
    ];
    let mut cmd_kitty = KittyCommand::new(&payload, ctrl_data);
    let resp = execute_and_read(&mut cmd_kitty)?;
    println!("Kitty Resp: {} ({resp:?})", resp_to_str(&resp, &cmd_kitty)?);

    Ok(())
}
