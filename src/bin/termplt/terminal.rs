//! Checks that the plot can be shown in the terminal and finds its size.
//!
//! The decisions live in [`prepare`], which reaches the terminal and OS only through the
//! [`Terminal`] trait so each branch can be tested without a real terminal.

use crate::Result;
use std::io::{self, IsTerminal, Write};
use termplt::{
    Error, WindowSize,
    terminal_commands::{
        kitty_cmds::{self, Passthrough},
        responses::TerminalCommandError,
    },
};

/// Cell size assumed when the terminal's pixel size is unknown (a common size for 12-14 pt
/// fonts).
pub const ASSUMED_CELL_SIZE: (u32, u32) = (9, 18);

/// What [`prepare`] needs from the terminal and the OS.
pub trait Terminal {
    fn stdout_is_terminal(&self) -> bool;
    fn passthrough(&self) -> Passthrough;
    /// See [`kitty_cmds::query_support`].
    fn query_support(&self) -> termplt::Result<()>;
    /// Whether tmux would drop the image because `allow-passthrough` is off.
    fn tmux_passthrough_disabled(&self) -> bool;
    /// See [`termplt::get_window_size`].
    fn window_size(&self) -> termplt::Result<WindowSize>;
    /// The size in cells, as (cols, rows).
    fn cell_count(&self) -> Option<(u16, u16)>;
}

/// The real terminal attached to this process.
pub struct Tty;

impl Terminal for Tty {
    fn stdout_is_terminal(&self) -> bool {
        io::stdout().is_terminal()
    }

    fn passthrough(&self) -> Passthrough {
        Passthrough::detect()
    }

    fn query_support(&self) -> termplt::Result<()> {
        kitty_cmds::query_support()
    }

    fn tmux_passthrough_disabled(&self) -> bool {
        std::process::Command::new("tmux")
            .args(["show-options", "-pAv", "allow-passthrough"])
            .stderr(std::process::Stdio::null())
            .output()
            .is_ok_and(|out| {
                out.status.success() && passthrough_off(&String::from_utf8_lossy(&out.stdout))
            })
    }

    fn window_size(&self) -> termplt::Result<WindowSize> {
        termplt::get_window_size()
    }

    fn cell_count(&self) -> Option<(u16, u16)> {
        crossterm::terminal::size().ok()
    }
}

/// Interprets `tmux show-options -pAv allow-passthrough`. The option is `off` by default, in
/// which case tmux silently drops the image; versions before 3.3 have no such option (the
/// command fails) and always pass sequences on.
fn passthrough_off(option_value: &str) -> bool {
    option_value.trim() == "off"
}

/// Checks that stdout is a terminal that can show images and returns its size. Warnings and
/// verbose details are written to `log`.
pub fn prepare(term: &impl Terminal, verbose: bool, log: &mut impl Write) -> Result<WindowSize> {
    if !term.stdout_is_terminal() {
        return Err(
            "stdout is not a terminal. termplt draws plots using the Kitty graphics \
                    protocol and must write to a terminal that supports it (e.g. Kitty, \
                    WezTerm, Ghostty); use --output plot.png to write an image file instead."
                .into(),
        );
    }

    // inside tmux the outer terminal's reply never reaches us, so there is nothing to query
    match term.passthrough() {
        Passthrough::None => match term.query_support() {
            Ok(()) => {
                if verbose {
                    writeln!(
                        log,
                        "[verbose] terminal supports the Kitty graphics protocol"
                    )?;
                }
            }
            Err(e @ (Error::GraphicsUnsupported | Error::GraphicsRejected(_))) => {
                return Err(
                    format!("{e}. Use --output plot.png to write an image file instead.").into(),
                );
            }
            // a terminal that answers nothing at all may still draw images; try anyway
            Err(Error::Terminal(TerminalCommandError::Timeout(_))) => writeln!(
                log,
                "warning: the terminal did not answer a graphics support query; drawing anyway"
            )?,
            Err(e) => writeln!(log, "warning: could not check for graphics support: {e}")?,
        },
        Passthrough::Tmux => {
            if term.tmux_passthrough_disabled() {
                return Err("tmux is blocking the image: enable passthrough with \
                            `tmux set -g allow-passthrough on` (add `set -g allow-passthrough \
                            on` to ~/.tmux.conf to keep it), or use --output plot.png to write \
                            an image file instead."
                    .into());
            }
            if verbose {
                writeln!(
                    log,
                    "[verbose] inside tmux: sending images through tmux passthrough"
                )?;
            }
        }
    }

    let window = match term.window_size() {
        Ok(window) => window,
        Err(e) => {
            let window = term
                .cell_count()
                .and_then(|(cols, rows)| estimate_window_size(cols, rows))
                .ok_or(e)?;
            writeln!(
                log,
                "warning: the terminal did not report its size in pixels; assuming {}x{} pixel \
                 cells. Use --width/--height to set the plot size.",
                window.pix_per_col, window.pix_per_row
            )?;
            window
        }
    };
    if verbose {
        writeln!(
            log,
            "[verbose] terminal: {}x{} cells, {}x{} pixels ({} px/col, {} px/row)",
            window.cols,
            window.rows,
            window.x_pix,
            window.y_pix,
            window.pix_per_col,
            window.pix_per_row
        )?;
    }
    Ok(window)
}

/// A window size from the cell count alone, for terminals that don't report pixel sizes.
fn estimate_window_size(cols: u16, rows: u16) -> Option<WindowSize> {
    let (cols, rows) = (u32::from(cols), u32::from(rows));
    if cols == 0 || rows == 0 {
        return None;
    }
    let (pix_per_col, pix_per_row) = ASSUMED_CELL_SIZE;
    Some(WindowSize {
        rows,
        cols,
        x_pix: cols * pix_per_col,
        y_pix: rows * pix_per_row,
        pix_per_col,
        pix_per_row,
    })
}

/// Rows an image of `height` pixels covers, i.e. how far to move the cursor to get below it.
pub fn rows_covered(height: u32, window: &WindowSize) -> u32 {
    height.div_ceil(window.pix_per_row.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, time::Duration};

    /// Scripted answers; counts the calls that talk to the terminal.
    struct FakeTerminal {
        stdout_is_terminal: bool,
        passthrough: Passthrough,
        support: fn() -> termplt::Result<()>,
        passthrough_disabled: bool,
        window_size: fn() -> termplt::Result<WindowSize>,
        cell_count: Option<(u16, u16)>,
        queries: Cell<u32>,
        tmux_checks: Cell<u32>,
    }

    impl Default for FakeTerminal {
        fn default() -> Self {
            FakeTerminal {
                stdout_is_terminal: true,
                passthrough: Passthrough::None,
                support: || Ok(()),
                passthrough_disabled: false,
                window_size: || Ok(window()),
                cell_count: Some((160, 50)),
                queries: Cell::new(0),
                tmux_checks: Cell::new(0),
            }
        }
    }

    impl Terminal for FakeTerminal {
        fn stdout_is_terminal(&self) -> bool {
            self.stdout_is_terminal
        }

        fn passthrough(&self) -> Passthrough {
            self.passthrough
        }

        fn query_support(&self) -> termplt::Result<()> {
            self.queries.set(self.queries.get() + 1);
            (self.support)()
        }

        fn tmux_passthrough_disabled(&self) -> bool {
            self.tmux_checks.set(self.tmux_checks.get() + 1);
            self.passthrough_disabled
        }

        fn window_size(&self) -> termplt::Result<WindowSize> {
            (self.window_size)()
        }

        fn cell_count(&self) -> Option<(u16, u16)> {
            self.cell_count
        }
    }

    fn window() -> WindowSize {
        WindowSize {
            rows: 50,
            cols: 160,
            x_pix: 1600,
            y_pix: 1000,
            pix_per_col: 10,
            pix_per_row: 20,
        }
    }

    fn no_window_size() -> termplt::Result<WindowSize> {
        Err(Error::Terminal(TerminalCommandError::InvalidResponse(
            "no reply to CSI 14t".into(),
        )))
    }

    /// Runs `prepare`, returning its result and everything it logged.
    fn run(term: &FakeTerminal, verbose: bool) -> (Result<WindowSize>, String) {
        let mut log = Vec::new();
        let result = prepare(term, verbose, &mut log);
        (result, String::from_utf8(log).unwrap())
    }

    #[test]
    fn supported_terminal_uses_reported_size_silently() {
        let term = FakeTerminal::default();
        let (result, log) = run(&term, false);
        assert_eq!(result.unwrap().x_pix, 1600);
        assert_eq!(log, "");
        assert_eq!(term.queries.get(), 1);
        assert_eq!(term.tmux_checks.get(), 0);
    }

    #[test]
    fn verbose_reports_support_and_size() {
        let (result, log) = run(&FakeTerminal::default(), true);
        assert!(result.is_ok());
        assert!(log.contains("supports the Kitty graphics protocol"));
        assert!(log.contains("160x50 cells, 1600x1000 pixels (10 px/col, 20 px/row)"));
    }

    #[test]
    fn stdout_not_a_terminal_is_an_error_before_any_query() {
        let term = FakeTerminal {
            stdout_is_terminal: false,
            ..Default::default()
        };
        let err = run(&term, false).0.unwrap_err().to_string();
        assert!(err.contains("stdout is not a terminal"));
        assert!(err.contains("--output"));
        assert_eq!(term.queries.get(), 0);
    }

    #[test]
    fn unsupported_terminal_is_an_error_with_output_hint() {
        let term = FakeTerminal {
            support: || Err(Error::GraphicsUnsupported),
            ..Default::default()
        };
        let err = run(&term, false).0.unwrap_err().to_string();
        assert!(err.contains("does not support the Kitty graphics protocol"));
        assert!(err.contains("--output plot.png"));
    }

    #[test]
    fn rejected_test_image_is_an_error() {
        let term = FakeTerminal {
            support: || Err(Error::GraphicsRejected("EINVAL".into())),
            ..Default::default()
        };
        let err = run(&term, false).0.unwrap_err().to_string();
        assert!(err.contains("EINVAL"));
    }

    #[test]
    fn silent_terminal_warns_and_continues() {
        let term = FakeTerminal {
            support: || {
                Err(Error::Terminal(TerminalCommandError::Timeout(
                    Duration::from_secs(2),
                )))
            },
            ..Default::default()
        };
        let (result, log) = run(&term, false);
        assert!(result.is_ok());
        assert!(log.contains("did not answer a graphics support query; drawing anyway"));
    }

    #[test]
    fn other_query_failures_warn_and_continue() {
        let term = FakeTerminal {
            support: || Err(Error::Io(std::io::Error::other("terminal input closed"))),
            ..Default::default()
        };
        let (result, log) = run(&term, false);
        assert!(result.is_ok());
        assert!(log.contains("could not check for graphics support: terminal input closed"));
    }

    #[test]
    fn tmux_skips_the_query_and_checks_passthrough() {
        let term = FakeTerminal {
            passthrough: Passthrough::Tmux,
            ..Default::default()
        };
        let (result, log) = run(&term, true);
        assert!(result.is_ok());
        assert!(log.contains("tmux passthrough"));
        assert_eq!(term.queries.get(), 0);
        assert_eq!(term.tmux_checks.get(), 1);
    }

    #[test]
    fn tmux_with_passthrough_off_is_an_error_with_the_fix() {
        let term = FakeTerminal {
            passthrough: Passthrough::Tmux,
            passthrough_disabled: true,
            ..Default::default()
        };
        let err = run(&term, false).0.unwrap_err().to_string();
        assert!(err.contains("tmux set -g allow-passthrough on"));
    }

    #[test]
    fn missing_pixel_size_is_estimated_with_a_warning() {
        let term = FakeTerminal {
            window_size: no_window_size,
            cell_count: Some((100, 40)),
            ..Default::default()
        };
        let (result, log) = run(&term, false);
        let window = result.unwrap();
        assert_eq!((window.cols, window.rows), (100, 40));
        assert_eq!((window.x_pix, window.y_pix), (900, 720));
        assert!(log.contains("assuming 9x18 pixel cells"));
    }

    #[test]
    fn no_size_at_all_returns_the_original_error() {
        let term = FakeTerminal {
            window_size: no_window_size,
            cell_count: None,
            ..Default::default()
        };
        let err = run(&term, false).0.unwrap_err().to_string();
        assert!(err.contains("no reply to CSI 14t"));

        // a zero-sized report is as good as none
        let term = FakeTerminal {
            window_size: no_window_size,
            cell_count: Some((0, 0)),
            ..Default::default()
        };
        assert!(run(&term, false).0.is_err());
    }

    #[test]
    fn tmux_option_parsing() {
        assert!(passthrough_off("off\n"));
        assert!(!passthrough_off("on\n"));
        assert!(!passthrough_off("all\n"));
        assert!(!passthrough_off(""));
    }

    #[test]
    fn rows_covered_rounds_up() {
        assert_eq!(rows_covered(600, &window()), 30);
        assert_eq!(rows_covered(601, &window()), 31);
        let zero_rows = WindowSize {
            pix_per_row: 0,
            ..window()
        };
        assert_eq!(rows_covered(20, &zero_rows), 20);
    }
}
