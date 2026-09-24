use super::responses::{TermCommand, TerminalCommandError};
use crate::kitty_graphics::ctrl_seq::{CtrlSeq, Metadata};
use crate::kitty_graphics::encoding;
use crate::{Error, common::Result};

const CMD_START: &[u8] = b"\x1B_G";
const CMD_SEP: &[u8] = b";";
const CMD_END: &[u8] = b"\x1B\\";
const MAX_PAYLOAD_SIZE: usize = 4096;

const TMUX_START: &[u8] = b"\x1BPtmux;";
const ESC: u8 = 0x1B;

/// Image id used by the support query. The query transmits nothing that is kept, so the id does
/// not collide with images the caller displays.
const QUERY_ID: u32 = 31;

/// How graphics commands reach the terminal that draws them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Passthrough {
    /// Commands are written as-is.
    None,
    /// Running inside tmux: each command is wrapped in a DCS passthrough sequence so tmux
    /// forwards it to the outer terminal. Requires `set -g allow-passthrough on` in tmux.
    Tmux,
}

impl Passthrough {
    /// Detects a terminal multiplexer from the environment (`$TMUX` is set inside tmux).
    pub fn detect() -> Passthrough {
        match std::env::var_os("TMUX") {
            Some(v) if !v.is_empty() => Passthrough::Tmux,
            _ => Passthrough::None,
        }
    }

    fn wrap(self, seq: &[u8], out: &mut Vec<u8>) {
        match self {
            Passthrough::None => out.extend_from_slice(seq),
            Passthrough::Tmux => {
                // tmux ends the passthrough at the first ESC \, so every ESC inside is doubled
                out.extend_from_slice(TMUX_START);
                for &b in seq {
                    if b == ESC {
                        out.push(ESC);
                    }
                    out.push(b);
                }
                out.extend_from_slice(CMD_END);
            }
        }
    }
}

pub struct KittyCommand {
    cmd: Vec<u8>,
}

impl TermCommand for KittyCommand {
    fn get_request(&self) -> &[u8] {
        &self.cmd
    }

    fn get_response_start(&self) -> &[u8] {
        CMD_START
    }

    fn get_response_end(&self) -> &[u8] {
        CMD_END
    }
}

impl KittyCommand {
    /// Builds the command, wrapped for the multiplexer detected by [`Passthrough::detect`].
    pub fn new(payload: &[u8], ctrl_data: &[String]) -> KittyCommand {
        KittyCommand::with_passthrough(payload, ctrl_data, Passthrough::detect())
    }

    pub fn with_passthrough(
        payload: &[u8],
        ctrl_data: &[String],
        passthrough: Passthrough,
    ) -> KittyCommand {
        let payload = encoding::read_bytes_to_b64(payload)
            .expect("base64 encoding of a byte slice cannot fail");
        let mut ctrl_data = Vec::from(ctrl_data);

        let chunks = payload.chunks(MAX_PAYLOAD_SIZE);
        let num_chunks = chunks.len();

        let mut cmd = Vec::with_capacity((MAX_PAYLOAD_SIZE + 64) * num_chunks);
        let mut seq = Vec::with_capacity(MAX_PAYLOAD_SIZE + 64);
        for (ind, chunk) in chunks.enumerate() {
            let is_last = ind == num_chunks - 1;

            ctrl_data.push(Metadata::MoreData(!is_last).get_ctrl_seq());
            let ctrl_bytes = std::mem::take(&mut ctrl_data).join(",");

            seq.clear();
            seq.extend_from_slice(CMD_START);
            seq.extend_from_slice(ctrl_bytes.as_bytes());
            seq.extend_from_slice(CMD_SEP);
            seq.extend_from_slice(chunk);
            seq.extend_from_slice(CMD_END);
            passthrough.wrap(&seq, &mut cmd);
        }

        KittyCommand { cmd }
    }
}

/// Asks the terminal whether it supports the Kitty graphics protocol, by sending a query for a
/// 1x1 image (nothing is stored or displayed) followed by the DA1 sentinel.
///
/// Returns [`Error::GraphicsUnsupported`] or [`Error::GraphicsRejected`] when the terminal
/// answers without supporting the protocol.
/// Other errors (no terminal, or a terminal that answers nothing at all) are passed through, so
/// callers can decide whether to try drawing anyway. Inside tmux the outer terminal's reply
/// does not reach the program, so callers should skip the query there.
pub fn query_support() -> Result<()> {
    let ctrl = [
        format!("i={QUERY_ID}"),
        String::from("a=q"),
        String::from("s=1"),
        String::from("v=1"),
        String::from("f=24"),
        String::from("t=d"),
    ];
    let cmd = KittyCommand::with_passthrough(&[0, 0, 0], &ctrl, Passthrough::None);
    match cmd.execute_with_response() {
        Ok(reply) => parse_query_reply(&reply),
        Err(Error::Terminal(TerminalCommandError::Unsupported)) => Err(Error::GraphicsUnsupported),
        Err(e) => Err(e),
    }
}

/// Interprets the body of a query reply, e.g. `i=31;OK` or `i=31;EINVAL:bad data`.
fn parse_query_reply(reply: &str) -> Result<()> {
    let msg = reply.split_once(';').map_or(reply, |(_, msg)| msg);
    if msg == "OK" {
        Ok(())
    } else {
        Err(Error::GraphicsRejected(msg.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctrl(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn single_chunk_command() {
        let cmd = KittyCommand::with_passthrough(&[0, 0, 0], &ctrl(&["a=T"]), Passthrough::None);
        assert_eq!(cmd.get_request(), b"\x1b_Ga=T,m=0;AAAA\x1b\\");
    }

    #[test]
    fn large_payload_is_chunked_with_attributes_on_first_chunk() {
        // 3072 bytes encode to exactly 4096 base64 characters
        let cmd = KittyCommand::with_passthrough(
            &[0; 3072 + 3],
            &ctrl(&["a=T", "f=24"]),
            Passthrough::None,
        );
        let req = cmd.get_request();
        let chunks: Vec<&[u8]> = req
            .split(|b| *b == ESC)
            .filter(|c| c.starts_with(b"_G"))
            .collect();
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].starts_with(b"_Ga=T,f=24,m=1;"));
        assert_eq!(chunks[1], b"_Gm=0;AAAA");
    }

    #[test]
    fn tmux_passthrough_wraps_each_chunk_and_doubles_escapes() {
        let cmd = KittyCommand::with_passthrough(&[0, 0, 0], &ctrl(&["a=T"]), Passthrough::Tmux);
        assert_eq!(
            cmd.get_request(),
            b"\x1bPtmux;\x1b\x1b_Ga=T,m=0;AAAA\x1b\x1b\\\x1b\\"
        );

        let cmd = KittyCommand::with_passthrough(&[0; 3075], &ctrl(&["a=T"]), Passthrough::Tmux);
        let req = cmd.get_request();
        let wrapped = req.windows(TMUX_START.len()).filter(|w| *w == TMUX_START);
        assert_eq!(wrapped.count(), 2);
    }

    #[test]
    fn query_reply_parsing() {
        assert!(parse_query_reply("i=31;OK").is_ok());
        let err = parse_query_reply("i=31;ENOTSUPPORTED:no").unwrap_err();
        assert!(err.to_string().contains("ENOTSUPPORTED:no"));
    }
}
