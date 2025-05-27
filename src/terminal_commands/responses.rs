use crossterm::terminal;
use std::io::{self, Read, Write};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn execute_csi(csi_cmd: &str) -> Result<Vec<u8>> {
    terminal::enable_raw_mode()?;

    {
        let mut stdout = io::stdout().lock();
        write!(stdout, "\x1b[{csi_cmd}")?;
        stdout.flush()?;
    }

    let mut stdin = std::io::stdin().lock();
    let mut buf = Vec::<u8>::new();
    let mut byte_buf = [0u8; 1];

    loop {
        match stdin.read_exact(&mut byte_buf) {
            Ok(_) => {
                buf.push(byte_buf[0]);

                if buf.len() > 3 && buf.ends_with(b"R") {
                    if buf.starts_with(b"\x1b") {
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

pub fn read_command() -> Result<()> {
    let csi_resp = execute_csi("6n")?;
    println!("CSI Seq: {csi_resp:?}");
    Ok(())
}
