use crossterm::{event, terminal};
use std::io::{self, Read, Write};

pub fn read_command() -> Result<(), Box<dyn std::error::Error>> {
    terminal::enable_raw_mode()?;

    let mut stdout = std::io::stdout().lock();

    write!(stdout, "\x1b[6n")?;

    stdout.flush()?;

    let mut stdin = std::io::stdin().lock();
    let mut buf = Vec::<u8>::new();
    let mut byte_buf = [0u8; 1];

    loop {
        match stdin.read_exact(&mut byte_buf) {
            Ok(_) => {
                buf.push(byte_buf[0]);

                if buf.len() > 3 && buf.ends_with(b"R") {
                    break;
                }
            }
            _ => (),
        }
    }

    Ok(())
}

