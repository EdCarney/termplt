use crossterm::terminal;
use std::io::{self, Read, Write};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn execute_csi(cmd: &str) -> Result<String> {
    let resp_start = "\x1b";
    let resp_end = "R";
    let cmd = String::from("\x1b[") + cmd;
    let resp = execute_and_read(&cmd, resp_start, resp_end)?;
    resp_to_str(resp, resp_start, resp_end)
}

pub fn execute_kitty(cmd: &str) -> Result<String> {
    let resp_start = "\x1b_G";
    let resp_end = "\x1b\\";
    let cmd = String::from(resp_start) + cmd + resp_end;
    let resp = execute_and_read(&cmd, resp_start, resp_end)?;
    resp_to_str(resp, resp_start, resp_end)
}

fn execute_and_read(cmd: &str, resp_start: &str, resp_end: &str) -> Result<Vec<u8>> {
    terminal::enable_raw_mode()?;

    {
        let mut stdout = io::stdout().lock();
        write!(stdout, "{cmd}")?;
        stdout.flush()?;
    }

    let mut stdin = std::io::stdin().lock();
    let mut buf = Vec::<u8>::new();
    let mut byte_buf = [0u8; 1];

    loop {
        match stdin.read_exact(&mut byte_buf) {
            Ok(_) => {
                buf.push(byte_buf[0]);
                if buf.len() > 3 && buf.ends_with(resp_end.as_bytes()) {
                    if buf.starts_with(resp_start.as_bytes()) {
                        break;
                    }
                    buf.clear();
                }
            }
            _ => (),
        }
    }

    terminal::disable_raw_mode()?;
    Ok(buf)
}

fn resp_to_str(resp: Vec<u8>, resp_start: &str, resp_end: &str) -> Result<String> {
    assert!(resp.len() >= resp_start.len() + resp_end.len());

    let net_resp_len = resp.len() - resp_start.len() - resp_end.len();
    let resp = resp
        .into_iter()
        .skip(resp_start.len())
        .take(net_resp_len)
        .collect();
    let resp = String::from_utf8(resp)?;
    Ok(resp)
}

pub fn read_command() -> Result<()> {
    let resp = execute_csi("6n")?;
    println!("CSI Resp: {resp:?}");

    let resp = execute_kitty("i=31,s=1,v=1,a=q,d=t,f=24;AAAA")?;
    println!("Kitty Resp: {resp:?}");
    Ok(())
}

