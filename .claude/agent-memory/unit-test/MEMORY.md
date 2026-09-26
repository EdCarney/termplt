# termplt Unit Test Memory

CLAUDE.md ("Testing") is the source of truth; this is a summary for writing new tests.

## Layout
- Unit tests are co-located in modules (`#[cfg(test)] mod tests`, `use super::*`).
- `tests/golden.rs`: rendered scenes compared with `tests/snapshots/*.png`.
- `tests/properties.rs`: proptest (no panics on arbitrary data; scaled points stay in bounds).
- `tests/cli.rs`: runs the CLI binary with piped stdio (needs the `cli` feature).
- `tests/pty.rs` (Unix): runs the CLI in a pseudo-terminal against a scripted fake terminal.

## Fakes for terminal code
- `src/terminal.rs`: a fake `Backend` scripts TTY/tmux/support/size answers for
  `Terminal::connect_to`.
- `src/terminal_commands/responses.rs`: a fake `ByteSource` replays reply chunks for
  `read_response` (split replies, DA1 sentinel, timeouts, key presses).

## Conventions
- Errors are `termplt::Error` variants; match them with `matches!` rather than comparing strings.
- Files in tests go in a `tempfile::tempdir()`, never fixed names in the shared temp directory.
- Run `cargo test` in debug too; release builds hide integer overflow.
