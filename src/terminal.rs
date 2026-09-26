//! Showing images in the terminal: checking that it can, finding its size, and placing images.
//!
//! [`Terminal::connect`] runs the checks once; the returned [`Terminal`] then displays images.
//!
//! ```no_run
//! use termplt::terminal::Terminal;
//!
//! let terminal = Terminal::connect()?;
//! let (width, height) = terminal.default_plot_size();
//! # let rgb = vec![0u8; (width * height * 3) as usize];
//! terminal.show_rgb(&rgb, width, height)?;
//! # Ok::<(), termplt::Error>(())
//! ```

use crate::{Error, Result, plotting::text::MAX_FONT_SIZE};
use std::io::{self, IsTerminal, Write};

pub use crate::{
    kitty_graphics::ctrl_seq::{PixelFormat, Transmission},
    terminal_commands::{
        images::{Image, PositioningType},
        kitty_cmds::{Passthrough, query_support},
        responses::TerminalCommandError,
    },
    window_ctrl::{WindowSize, get_window_size},
};

/// Cell size assumed when the terminal's pixel size is unknown (a common size for 12-14 pt
/// fonts).
pub const ASSUMED_CELL_SIZE: (u32, u32) = (9, 18);

/// Window size in cells (columns, rows) assumed when the terminal reports no size at all.
pub const ASSUMED_CELL_COUNT: (u16, u16) = (80, 24);

/// Smallest size [`Terminal::default_plot_size`] returns.
pub const MIN_PLOT_SIZE: (u32, u32) = (200, 150);

/// A terminal that has been checked for graphics support, with its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Terminal {
    window: WindowSize,
    passthrough: Passthrough,
}

impl Terminal {
    /// Checks that stdout is a terminal that can show images and finds its size.
    ///
    /// Fails with [`Error::NotATerminal`], [`Error::GraphicsUnsupported`],
    /// [`Error::GraphicsRejected`] or [`Error::TmuxPassthroughDisabled`] when images cannot be
    /// shown. A terminal that answers no queries at all is assumed to work, a terminal that
    /// doesn't report its size in pixels gets an estimate based on [`ASSUMED_CELL_SIZE`], and one
    /// that reports no size at all is assumed to be [`ASSUMED_CELL_COUNT`] cells; use
    /// [`Terminal::connect_with_log`] to see those warnings.
    pub fn connect() -> Result<Terminal> {
        Terminal::connect_with_log(false, &mut io::sink())
    }

    /// Like [`Terminal::connect`], writing warnings (and details when `verbose`) to `log`.
    pub fn connect_with_log(verbose: bool, log: &mut impl Write) -> Result<Terminal> {
        Terminal::connect_to(&Tty, verbose, log)
    }

    fn connect_to(term: &impl Backend, verbose: bool, log: &mut impl Write) -> Result<Terminal> {
        if !term.stdout_is_terminal() {
            return Err(Error::NotATerminal);
        }

        // inside tmux the outer terminal's reply never reaches us, so there is nothing to query
        let passthrough = term.passthrough();
        match passthrough {
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
                    return Err(e);
                }
                // a terminal that answers nothing at all may still draw images; try anyway
                Err(Error::Terminal(TerminalCommandError::Timeout(_))) => writeln!(
                    log,
                    "warning: the terminal did not answer a graphics support query; drawing \
                     anyway"
                )?,
                Err(e) => writeln!(log, "warning: could not check for graphics support: {e}")?,
            },
            Passthrough::Tmux => {
                if term.tmux_passthrough_disabled() {
                    return Err(Error::TmuxPassthroughDisabled);
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
            Err(_) => match term
                .cell_count()
                .and_then(|(cols, rows)| estimate_window_size(cols, rows))
            {
                Some(window) => {
                    writeln!(
                        log,
                        "warning: the terminal did not report its size in pixels; assuming \
                         {}x{} pixel cells",
                        window.pix_per_col, window.pix_per_row
                    )?;
                    window
                }
                // like a terminal that answers no graphics query, try anyway
                None => {
                    let (cols, rows) = ASSUMED_CELL_COUNT;
                    let window = estimate_window_size(cols, rows)
                        .expect("the assumed cell count is non-zero");
                    writeln!(
                        log,
                        "warning: the terminal did not report its size; assuming {cols}x{rows} \
                         cells of {}x{} pixels",
                        window.pix_per_col, window.pix_per_row
                    )?;
                    window
                }
            },
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
        Ok(Terminal {
            window,
            passthrough,
        })
    }

    /// The terminal's size.
    pub fn window(&self) -> &WindowSize {
        &self.window
    }

    /// How images reach the terminal.
    pub fn passthrough(&self) -> Passthrough {
        self.passthrough
    }

    /// A plot size that fits the window: the full width (less one column, so the image does not
    /// wrap) and 60% of the height, at most twice as wide as high and at least
    /// [`MIN_PLOT_SIZE`].
    pub fn default_plot_size(&self) -> (u32, u32) {
        default_plot_size(&self.window)
    }

    /// The base text size in pixels that matches the terminal's own text: a terminal row is
    /// about 1.2 em, so the row height divided by 1.2, at least 8 and at most
    /// [`MAX_FONT_SIZE`].
    pub fn text_size(&self) -> u32 {
        text_size_for(&self.window)
    }

    /// Displays an image at the cursor and moves the cursor below it.
    pub fn show(&self, image: &Image) -> Result<()> {
        let mut stdout = io::stdout();
        match self.passthrough {
            Passthrough::Tmux => {
                // tmux doesn't know the image is there, so it wouldn't account for the terminal
                // moving the cursor below it; move the cursor ourselves instead
                image.display_without_moving_cursor()?;
                let rows = rows_covered(image.height(), &self.window);
                write!(stdout, "{}", "\n".repeat(rows as usize))?;
            }
            Passthrough::None => {
                image.display()?;
                // the terminal leaves the cursor on the image's last row
                writeln!(stdout)?;
            }
        }
        stdout.flush()?;
        Ok(())
    }

    /// PNG-encodes RGB8 pixels (`width * height * 3` bytes) and displays them like
    /// [`Terminal::show`].
    pub fn show_rgb(&self, rgb: &[u8], width: u32, height: u32) -> Result<()> {
        self.show(&Image::png_from_rgb(rgb, width, height)?)
    }
}

fn default_plot_size(window: &WindowSize) -> (u32, u32) {
    // leave one column free so the image does not wrap
    let available_w = window.x_pix.saturating_sub(window.pix_per_col);
    let h = (window.y_pix * 3 / 5).max(MIN_PLOT_SIZE.1);
    let w = available_w.min(2 * h).max(MIN_PLOT_SIZE.0);
    (w, h)
}

/// What [`Terminal::connect_to`] needs from the terminal and the OS, so every branch can be
/// tested without a real terminal.
trait Backend {
    fn stdout_is_terminal(&self) -> bool;
    fn passthrough(&self) -> Passthrough;
    fn query_support(&self) -> Result<()>;
    /// Whether tmux would drop the image because `allow-passthrough` is off.
    fn tmux_passthrough_disabled(&self) -> bool;
    fn window_size(&self) -> Result<WindowSize>;
    /// The size in cells, as (cols, rows).
    fn cell_count(&self) -> Option<(u16, u16)>;
}

/// The real terminal attached to this process.
struct Tty;

impl Backend for Tty {
    fn stdout_is_terminal(&self) -> bool {
        io::stdout().is_terminal()
    }

    fn passthrough(&self) -> Passthrough {
        Passthrough::detect()
    }

    fn query_support(&self) -> Result<()> {
        query_support()
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

    fn window_size(&self) -> Result<WindowSize> {
        get_window_size()
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

/// See [`Terminal::text_size`].
fn text_size_for(window: &WindowSize) -> u32 {
    ((window.pix_per_row as f32 / 1.2).round() as u32).clamp(8, MAX_FONT_SIZE)
}

#[cfg(test)]
impl Terminal {
    /// A terminal of a given size, for tests elsewhere in the crate.
    pub(crate) fn with_window(window: WindowSize) -> Terminal {
        Terminal {
            window,
            passthrough: Passthrough::None,
        }
    }
}

/// Rows an image of `height` pixels covers, i.e. how far to move the cursor to get below it.
fn rows_covered(height: u32, window: &WindowSize) -> u32 {
    height.div_ceil(window.pix_per_row.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, time::Duration};

    #[test]
    fn text_size_matches_the_terminal_rows() {
        let size = |pix_per_row| {
            text_size_for(&WindowSize {
                rows: 24,
                cols: 80,
                x_pix: 800,
                y_pix: 24 * pix_per_row,
                pix_per_row,
                pix_per_col: 10,
            })
        };
        assert_eq!(size(34), 28); // a 2x (Retina) terminal with 17 pt rows
        assert_eq!(size(17), 14);
        assert_eq!(size(9), 8); // never below 8 px
        assert_eq!(size(0), 8); // a terminal that reports no row height
        assert_eq!(size(1000), MAX_FONT_SIZE);
    }

    /// Scripted answers; counts the calls that talk to the terminal.
    struct FakeTerminal {
        stdout_is_terminal: bool,
        passthrough: Passthrough,
        support: fn() -> crate::Result<()>,
        passthrough_disabled: bool,
        window_size: fn() -> crate::Result<WindowSize>,
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

    impl Backend for FakeTerminal {
        fn stdout_is_terminal(&self) -> bool {
            self.stdout_is_terminal
        }

        fn passthrough(&self) -> Passthrough {
            self.passthrough
        }

        fn query_support(&self) -> crate::Result<()> {
            self.queries.set(self.queries.get() + 1);
            (self.support)()
        }

        fn tmux_passthrough_disabled(&self) -> bool {
            self.tmux_checks.set(self.tmux_checks.get() + 1);
            self.passthrough_disabled
        }

        fn window_size(&self) -> crate::Result<WindowSize> {
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

    fn no_window_size() -> crate::Result<WindowSize> {
        Err(Error::Terminal(TerminalCommandError::InvalidResponse(
            "no reply to CSI 14t".into(),
        )))
    }

    /// Runs `connect_to`, returning the window size and everything it logged.
    fn run(term: &FakeTerminal, verbose: bool) -> (Result<WindowSize>, String) {
        let mut log = Vec::new();
        let result = Terminal::connect_to(term, verbose, &mut log).map(|t| t.window);
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
        let err = run(&term, false).0.unwrap_err();
        assert!(matches!(err, Error::NotATerminal));
        assert_eq!(term.queries.get(), 0);
    }

    #[test]
    fn unsupported_terminal_is_an_error() {
        let term = FakeTerminal {
            support: || Err(Error::GraphicsUnsupported),
            ..Default::default()
        };
        let err = run(&term, false).0.unwrap_err();
        assert!(matches!(err, Error::GraphicsUnsupported));
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
        let err = run(&term, false).0.unwrap_err();
        assert!(matches!(err, Error::TmuxPassthroughDisabled));
        assert!(err.to_string().contains("tmux set -g allow-passthrough on"));
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
    fn no_size_at_all_assumes_a_standard_window() {
        // a zero-sized report is as good as none
        for cell_count in [None, Some((0, 0))] {
            let term = FakeTerminal {
                window_size: no_window_size,
                cell_count,
                ..Default::default()
            };
            let (window, log) = run(&term, false);
            let window = window.unwrap();
            assert_eq!((window.cols, window.rows), (80, 24));
            assert_eq!((window.x_pix, window.y_pix), (720, 432));
            assert!(
                log.contains("did not report its size; assuming 80x24 cells"),
                "{log}"
            );
        }
    }

    #[test]
    fn tmux_option_parsing() {
        assert!(passthrough_off("off\n"));
        assert!(!passthrough_off("on\n"));
        assert!(!passthrough_off("all\n"));
        assert!(!passthrough_off(""));
    }

    #[test]
    fn plot_size_fits_the_window() {
        let size = |x_pix, y_pix| {
            default_plot_size(&WindowSize {
                rows: y_pix / 20,
                cols: x_pix / 10,
                x_pix,
                y_pix,
                pix_per_col: 10,
                pix_per_row: 20,
            })
        };
        // wide terminal: 60% of the height, width capped at twice the height
        assert_eq!(size(1600, 1000), (1200, 600));
        // narrow terminal: full width minus one column
        assert_eq!(size(500, 1000), (490, 600));
        // tiny terminal: minimum size
        assert_eq!(size(100, 100), MIN_PLOT_SIZE);
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
