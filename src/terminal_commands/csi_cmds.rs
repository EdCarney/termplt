use super::responses::{TermCommand, TerminalCommandError};
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
    let resp = CsiCommand::new("6n", "R").execute_with_response()?;
    let vals = parse_numeric_response(&resp, 2, "cursor position")?;
    Ok(TermPosition {
        row: vals[0],
        col: vals[1],
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
    let resp = CsiCommand::new("14t", "t").execute_with_response()?;
    // response format: 4;height;width
    let vals = parse_numeric_response(&resp, 3, "text area size in pixels")?;
    check_prefix(&vals, 4, &resp)?;
    Ok((vals[2], vals[1]))
}

/// Query terminal text area size in character cells using xterm CSI 18 t.
/// Returns (rows, cols).
pub fn get_text_area_size_cells() -> Result<(u32, u32)> {
    let resp = CsiCommand::new("18t", "t").execute_with_response()?;
    // response format: 8;rows;cols
    let vals = parse_numeric_response(&resp, 3, "text area size in cells")?;
    check_prefix(&vals, 8, &resp)?;
    Ok((vals[1], vals[2]))
}

/// Parses a `;`-separated list of exactly `expected_len` unsigned integers.
fn parse_numeric_response(resp: &str, expected_len: usize, what: &str) -> Result<Vec<u32>> {
    let invalid = || TerminalCommandError::InvalidResponse(format!("{what}: {resp:?}"));
    let vals = resp
        .split(';')
        .map(|c| c.parse::<u32>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| invalid())?;
    if vals.len() != expected_len {
        return Err(invalid().into());
    }
    Ok(vals)
}

fn check_prefix(vals: &[u32], expected: u32, resp: &str) -> Result<()> {
    if vals[0] != expected {
        return Err(TerminalCommandError::InvalidResponse(format!(
            "expected reply type {expected}, got {resp:?}"
        ))
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_numeric_response_valid() {
        assert_eq!(
            parse_numeric_response("4;600;800", 3, "size").unwrap(),
            vec![4, 600, 800]
        );
    }

    #[test]
    fn parse_numeric_response_wrong_length_errors() {
        assert!(parse_numeric_response("4;600", 3, "size").is_err());
    }

    #[test]
    fn parse_numeric_response_non_numeric_errors() {
        assert!(parse_numeric_response("4;abc;800", 3, "size").is_err());
        assert!(parse_numeric_response("", 1, "size").is_err());
    }

    #[test]
    fn check_prefix_mismatch_errors() {
        assert!(check_prefix(&[8, 1, 2], 4, "8;1;2").is_err());
        assert!(check_prefix(&[4, 1, 2], 4, "4;1;2").is_ok());
    }
}
