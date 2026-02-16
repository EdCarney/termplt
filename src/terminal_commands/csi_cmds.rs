use super::responses::TermCommand;
use crate::common::Result;

const CMD_START: &[u8] = b"\x1b[";

#[derive(Debug)]
pub struct TermPosition {
    /// One-indexed row position with top row as index one.
    pub row: u32,
    /// One-indexed column position with leftmost column as index one.
    pub col: u32,
}

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

pub fn get_cursor_pos() -> Result<TermPosition> {
    let resp: Vec<u32> = CsiCommand::new("6n", "R")
        .execute_with_response()
        .expect("Error getting cursor position.")
        .split(';')
        .map(|c| c.parse::<u32>().expect("Failure parsing cursor position"))
        .collect();

    assert_eq!(
        resp.len(),
        2,
        "Expect cursor position return to have 2 elements"
    );

    Ok(TermPosition {
        row: resp[0],
        col: resp[1],
    })
}

pub fn set_cursor_pos(row: u32, col: u32) -> Result<()> {
    let cmd = format!("{row};{col}H");
    CsiCommand::new(&cmd, "").execute()
}

pub fn clear_screen() -> Result<()> {
    CsiCommand::new("2J", "").execute()
}

/// Query terminal text area size in pixels using xterm CSI 14 t.
/// Returns (width_px, height_px).
pub fn get_text_area_size_pixels() -> Result<(u32, u32)> {
    let resp: Vec<u32> = CsiCommand::new("14t", "t")
        .execute_with_response()?
        .split(';')
        .map(|c| c.parse::<u32>().expect("Failure parsing pixel size response"))
        .collect();

    assert_eq!(
        resp.len(),
        3,
        "Expect pixel size response to have 3 elements (prefix;height;width)"
    );

    Ok((resp[2], resp[1]))
}

/// Query terminal text area size in character cells using xterm CSI 18 t.
/// Returns (rows, cols).
pub fn get_text_area_size_cells() -> Result<(u32, u32)> {
    let resp: Vec<u32> = CsiCommand::new("18t", "t")
        .execute_with_response()?
        .split(';')
        .map(|c| c.parse::<u32>().expect("Failure parsing cell size response"))
        .collect();

    assert_eq!(
        resp.len(),
        3,
        "Expect cell size response to have 3 elements (prefix;rows;cols)"
    );

    Ok((resp[1], resp[2]))
}
