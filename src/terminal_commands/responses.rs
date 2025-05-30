use crossterm::terminal;
use std::{
    error::Error,
    fmt,
    io::{self, Read, Write},
    sync::Arc,
};

const CSI_START: &str = "\x1b[";
const KITTY_START: &str = "\x1b_G";
const KITTY_END: &str = "\x1b\\";

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

trait TermCommand {
    fn cmd(&self) -> String;
    fn req_start(&self) -> String {
        String::from("")
    }
    fn req_end(&self) -> String {
        String::from("")
    }
    fn res_start(&self) -> String {
        String::from("")
    }
    fn res_end(&self) -> String {
        String::from("")
    }
    fn generate_request(&self) -> Vec<u8> {
        let mut req = Vec::new();
        req.append(&mut self.req_start().as_bytes().to_vec());
        req.append(&mut self.cmd().as_bytes().to_vec());
        req.append(&mut self.req_end().as_bytes().to_vec());
        req
    }
}

#[derive(Clone)]
struct KittyCommand {
    cmd: String,
}

impl TermCommand for KittyCommand {
    fn cmd(&self) -> String {
        self.cmd.clone()
    }
    fn req_start(&self) -> String {
        String::from(KITTY_START)
    }
    fn req_end(&self) -> String {
        String::from(KITTY_END)
    }
    fn res_start(&self) -> String {
        String::from(KITTY_START)
    }
    fn res_end(&self) -> String {
        String::from(KITTY_END)
    }
}

#[derive(Clone)]
struct CsiCommand {
    cmd: String,
    res_end: String,
}

impl TermCommand for CsiCommand {
    fn cmd(&self) -> String {
        self.cmd.clone()
    }
    fn req_start(&self) -> String {
        String::from(CSI_START)
    }
    fn res_start(&self) -> String {
        String::from(CSI_START)
    }
    fn res_end(&self) -> String {
        self.res_end.clone()
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

fn execute_and_read<T: TermCommand>(cmd: &T) -> Result<Vec<u8>> {
    let (tx1, rx) = std::sync::mpsc::channel();
    let tx2 = tx1.clone();
    let res_start = cmd.res_start().into_bytes();
    let res_end = cmd.res_end().into_bytes();

    terminal::enable_raw_mode()?;

    std::thread::spawn(move || {
        let mut buf = Vec::<u8>::new();
        let mut stdin = std::io::stdin().lock();
        let mut byte_buf = [0u8; 1];

        loop {
            match stdin.read_exact(&mut byte_buf) {
                Ok(_) => {
                    buf.push(byte_buf[0]);
                    if buf.len() > res_start.len() && buf.ends_with(&res_end) {
                        if buf.starts_with(&res_start) {
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
        tx1.send(buf).unwrap();
    });

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        tx2.send(Vec::new()).unwrap_or_else(|_| ());
    });

    {
        let mut stdout = io::stdout().lock();
        stdout.write_all(cmd.generate_request().as_slice())?;
        stdout.flush()?;
    }

    let msg = rx.recv();
    terminal::disable_raw_mode()?;

    match msg {
        Ok(buf) if buf.len() > 0 => Ok(buf),
        _ => Err(Box::new(TerminalCommandError {
            failed_cmd: cmd.cmd(),
        })),
    }
}

fn resp_to_str<T: TermCommand>(resp: &[u8], cmd: &T) -> Result<String> {
    if resp.starts_with(cmd.res_start().as_bytes()) && resp.ends_with(cmd.res_end().as_bytes()) {
        let start = cmd.res_start().len();
        let end = resp.len() - cmd.res_end().len();
        Ok(String::from_utf8(resp[start..end].to_vec())?)
    } else {
        Err(Box::new(TerminalCommandError {
            failed_cmd: cmd.cmd(),
        }))
    }
}

pub fn read_command() -> Result<()> {
    let cmd = CsiCommand {
        cmd: String::from("6n"),
        res_end: String::from("R"),
    };
    let resp = execute_and_read(&cmd)?;
    println!("CSI Resp: {} ({resp:?})", resp_to_str(&resp, &cmd)?);

    let cmd = CsiCommand {
        cmd: String::from("c"),
        res_end: String::from("c"),
    };
    let resp = execute_and_read(&cmd)?;
    println!("CSI Resp: {} ({resp:?})", resp_to_str(&resp, &cmd)?);

    let cmd_kitty = KittyCommand {
        cmd: String::from("i=31,s=1,v=1,a=q,f=24;AAAA"),
    };
    let resp = execute_and_read(&cmd_kitty)?;
    println!("Kitty Resp: {} ({resp:?})", resp_to_str(&resp, &cmd_kitty)?);

    Ok(())
}
