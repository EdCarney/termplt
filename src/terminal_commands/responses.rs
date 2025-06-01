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
