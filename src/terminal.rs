//! Showing images in the terminal: checking that it can, finding its size, and placing images.
//!
//! [`Terminal::connect`] runs the checks once; the returned [`Terminal`] then displays images,
//! either once ([`Terminal::show`]) or as a [`Placement`] whose image can be replaced in place.
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

use crate::{
    Error, Result,
    plotting::text::MAX_FONT_SIZE,
    terminal_commands::{csi_cmds, images::delete_image_command},
};
use std::{
    io::{self, IsTerminal, Write},
    sync::{
        LazyLock,
        atomic::{AtomicU32, Ordering},
    },
};

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

    /// Shows `image` at the start of the cursor's line and returns a handle for replacing it in
    /// place, for example with the frames of a live plot.
    ///
    /// The image covers [`Placement::rows`] lines from the cursor's line down (the window
    /// scrolls if needed), and the cursor is left at the start of the line below it, as after
    /// every [`Placement::replace`]. Frames are drawn relative to the cursor, so nothing else may
    /// be written to the terminal while the image is being replaced. Images wider than the
    /// window are cut off on the right.
    ///
    /// Fails with [`Error::ImageTooTall`] when the image doesn't fit in the window with a line
    /// to spare for the cursor.
    ///
    /// ```no_run
    /// use termplt::terminal::{Image, Terminal};
    ///
    /// let terminal = Terminal::connect()?;
    /// let (width, height) = (300, 200);
    /// let mut rgb = vec![0u8; (width * height * 3) as usize];
    /// let mut placement = terminal.place(&Image::png_from_rgb(&rgb, width, height)?)?;
    /// for shade in 1..=255 {
    ///     rgb.fill(shade); // fades from black to white
    ///     placement.replace_rgb(&rgb, width, height)?;
    /// }
    /// # Ok::<(), termplt::Error>(())
    /// ```
    pub fn place(&self, image: &Image) -> Result<Placement> {
        Placement::new(*self, image, Sink::Stdout)
    }
}

/// An image shown by [`Terminal::place`], which can be replaced in place.
///
/// Each frame reaches the terminal in one write: the new image is sent under a new id while the
/// old one stays on screen, then shown over it, and then the old one is deleted, so nothing
/// flickers, even over a slow link. Dropping the handle sends nothing: the last frame stays on
/// screen with the cursor below it.
#[derive(Debug)]
pub struct Placement {
    terminal: Terminal,
    rows: u32,
    width: u32,
    height: u32,
    /// The high bits of this placement's image ids, from [`next_block`].
    block: u32,
    /// The number of the frame on screen, from 1.
    frame: u32,
    sink: Sink,
}

impl Placement {
    fn new(terminal: Terminal, image: &Image, mut sink: Sink) -> Result<Placement> {
        let rows = rows_covered(image.height(), &terminal.window);
        let screen_rows = terminal.window.rows;
        // the image must fit on the screen below the cursor's line, with a line left for the
        // cursor, or moving up to redraw it would stop at the top of the screen
        if rows > screen_rows.saturating_sub(1) {
            return Err(Error::ImageTooTall { rows, screen_rows });
        }
        let block = next_block();
        let first = frame_id(block, 1);
        sink.send(&frame_bytes(image, first, None, rows, terminal.passthrough))?;
        Ok(Placement {
            terminal,
            rows,
            width: image.width(),
            height: image.height(),
            block,
            frame: 1,
            sink,
        })
    }

    /// Replaces the image with `image`, which must have the same size in pixels (else
    /// [`Error::PlacementSize`]). Write errors, such as a closed terminal, are [`Error::Io`].
    pub fn replace(&mut self, image: &Image) -> Result<()> {
        self.check_size(image.width(), image.height())?;
        let next = self.frame.wrapping_add(1);
        let bytes = frame_bytes(
            image,
            frame_id(self.block, next),
            Some(frame_id(self.block, self.frame)),
            self.rows,
            self.terminal.passthrough,
        );
        self.sink.send(&bytes)?;
        // only once it was sent, so the next frame deletes the image that is on screen
        self.frame = next;
        Ok(())
    }

    /// PNG-encodes RGB8 pixels (`width * height * 3` bytes) and replaces the image with them,
    /// like [`Placement::replace`].
    pub fn replace_rgb(&mut self, rgb: &[u8], width: u32, height: u32) -> Result<()> {
        // before encoding, which is most of the work
        self.check_size(width, height)?;
        self.replace(&Image::png_from_rgb(rgb, width, height)?)
    }

    /// The image size in pixels, as (width, height).
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// The number of terminal lines the image covers.
    pub fn rows(&self) -> u32 {
        self.rows
    }

    fn check_size(&self, width: u32, height: u32) -> Result<()> {
        if (width, height) == self.size() {
            Ok(())
        } else {
            Err(Error::PlacementSize {
                expected: self.size(),
                got: (width, height),
            })
        }
    }
}

/// Where a [`Placement`] writes its frames.
#[derive(Debug)]
enum Sink {
    Stdout,
    /// Keeps the bytes, for tests.
    #[cfg(test)]
    Buffer(Vec<u8>),
    /// Fails every write, as a closed terminal does.
    #[cfg(test)]
    Closed,
}

impl Sink {
    /// Writes one frame with a single `write_all`, then flushes it.
    fn send(&mut self, frame: &[u8]) -> io::Result<()> {
        match self {
            Sink::Stdout => {
                let mut stdout = io::stdout().lock();
                stdout.write_all(frame)?;
                stdout.flush()
            }
            #[cfg(test)]
            Sink::Buffer(buffer) => {
                buffer.extend_from_slice(frame);
                Ok(())
            }
            #[cfg(test)]
            Sink::Closed => Err(io::ErrorKind::BrokenPipe.into()),
        }
    }
}

/// The bytes of one frame of a [`Placement`] covering `rows` lines, which shows `image` under
/// `id` and leaves the cursor at the start of the line below it. Only the graphics commands are
/// wrapped for `passthrough`; the newlines and cursor moves are written plainly, so tmux keeps
/// track of the cursor.
///
/// The first frame (no `previous` id) starts on the cursor's line: it reserves the lines below
/// it, scrolling if needed, and goes back up to show the image with its top-left corner at the
/// start of that line. A later frame starts below the image it replaces: the new image is sent
/// while the old one stays on screen, shown over it, and then the old one (`previous`) is
/// deleted with its data.
fn frame_bytes(
    image: &Image,
    id: u32,
    previous: Option<u32>,
    rows: u32,
    passthrough: Passthrough,
) -> Vec<u8> {
    let transmit = image.transmit_command(id, passthrough).into_bytes();
    let mut bytes = Vec::with_capacity(transmit.len() + 256);
    match previous {
        None => {
            bytes.push(b'\r');
            bytes.extend(std::iter::repeat_n(b'\n', rows as usize));
            bytes.extend(csi_cmds::cursor_up(rows));
            bytes.extend(transmit);
        }
        Some(_) => {
            bytes.extend(transmit);
            bytes.extend(csi_cmds::cursor_up(rows));
        }
    }
    bytes.extend(image.put_command(id, passthrough).into_bytes());
    if let Some(previous) = previous {
        bytes.extend(delete_image_command(previous, true, passthrough).into_bytes());
    }
    bytes.extend(csi_cmds::cursor_down(rows));
    bytes
}

/// Counts placements; seeded from the process id, so that two programs drawing in one terminal
/// (such as tmux panes) seldom use the same image ids.
static PLACEMENTS: LazyLock<AtomicU32> = LazyLock::new(|| AtomicU32::new(std::process::id()));

/// The id block (1 to 255) of a new placement.
fn next_block() -> u32 {
    id_block(PLACEMENTS.fetch_add(1, Ordering::Relaxed))
}

fn id_block(count: u32) -> u32 {
    1 + count % 255
}

/// The image id of frame `frame` (from 1) of the placement with id block `block`: the block
/// above the low 16 bits, which hold the frame number and wrap after 65535 frames. Ids are
/// never 0, stay below 2^24 (the most that Unicode placeholders can name), and rise from frame
/// to frame, so a new frame is on top for the instant that both frames are on screen.
fn frame_id(block: u32, frame: u32) -> u32 {
    (block << 16) | (frame % 0x1_0000)
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

    /// Like [`Terminal::place`], keeping what would be written; see [`Placement::written`].
    pub(crate) fn place_in_buffer(&self, image: &Image) -> Result<Placement> {
        Placement::new(*self, image, Sink::Buffer(Vec::new()))
    }
}

#[cfg(test)]
impl Placement {
    /// Everything written so far by a placement from [`Terminal::place_in_buffer`].
    pub(crate) fn written(&self) -> &[u8] {
        match &self.sink {
            Sink::Buffer(buffer) => buffer,
            _ => panic!("the placement writes to {:?}", self.sink),
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

    // -- placements --

    /// A raw RGB image, whose commands are easy to write out: 1x1 is one chunk (`AAAA`).
    fn rgb_image(width: u32, height: u32) -> Image {
        Image::new(
            PixelFormat::Rgb { width, height },
            Transmission::Direct(vec![0; (width * height * 3) as usize]),
        )
        .unwrap()
    }

    /// Wraps one escape sequence in a tmux passthrough, doubling its escapes.
    fn tmux_wrapped(seq: &[u8]) -> Vec<u8> {
        let mut out = b"\x1bPtmux;".to_vec();
        for &b in seq {
            if b == 0x1b {
                out.push(0x1b);
            }
            out.push(b);
        }
        out.extend_from_slice(b"\x1b\\");
        out
    }

    fn count(haystack: &[u8], needle: &[u8]) -> usize {
        haystack
            .windows(needle.len())
            .filter(|w| w == &needle)
            .count()
    }

    #[test]
    fn the_first_frame_reserves_the_lines_and_shows_the_image_at_their_start() {
        let bytes = frame_bytes(&rgb_image(1, 1), 65537, None, 3, Passthrough::None);
        assert_eq!(
            bytes,
            b"\r\n\n\n\x1b[3A\
              \x1b_Ga=t,f=24,s=1,v=1,t=d,i=65537,q=2,m=0;AAAA\x1b\\\
              \x1b_Ga=p,i=65537,C=1,q=2\x1b\\\
              \x1b[3B"
        );
    }

    #[test]
    fn a_later_frame_is_sent_first_and_deletes_the_one_it_covers() {
        let bytes = frame_bytes(&rgb_image(1, 1), 65538, Some(65537), 3, Passthrough::None);
        assert_eq!(
            bytes,
            b"\x1b_Ga=t,f=24,s=1,v=1,t=d,i=65538,q=2,m=0;AAAA\x1b\\\
              \x1b[3A\
              \x1b_Ga=p,i=65538,C=1,q=2\x1b\\\
              \x1b_Ga=d,d=I,i=65537,q=2\x1b\\\
              \x1b[3B"
        );
    }

    #[test]
    fn frames_in_tmux_wrap_only_the_graphics_commands() {
        let image = rgb_image(1, 1);
        let transmit = |id| {
            tmux_wrapped(format!("\x1b_Ga=t,f=24,s=1,v=1,t=d,i={id},q=2,m=0;AAAA\x1b\\").as_bytes())
        };
        let put = |id| tmux_wrapped(format!("\x1b_Ga=p,i={id},C=1,q=2\x1b\\").as_bytes());

        let mut first = b"\r\n\n\x1b[2A".to_vec();
        first.extend(transmit(65537));
        first.extend(put(65537));
        first.extend(b"\x1b[2B");
        assert_eq!(
            frame_bytes(&image, 65537, None, 2, Passthrough::Tmux),
            first
        );

        let mut next = transmit(65538);
        next.extend(b"\x1b[2A");
        next.extend(put(65538));
        next.extend(tmux_wrapped(b"\x1b_Ga=d,d=I,i=65537,q=2\x1b\\"));
        next.extend(b"\x1b[2B");
        assert_eq!(
            frame_bytes(&image, 65538, Some(65537), 2, Passthrough::Tmux),
            next
        );
    }

    #[test]
    fn frames_send_every_chunk_of_a_large_image() {
        // 2049 * 3 bytes are 8196 base64 characters: chunks of 4096, 4096 and 4
        let image = rgb_image(2049, 1);
        for passthrough in [Passthrough::None, Passthrough::Tmux] {
            let transmit = image.transmit_command(9, passthrough).into_bytes();
            assert_eq!(count(&transmit, b"m=1;"), 2);
            assert_eq!(count(&transmit, b"m=0;"), 1);
            let wrapped = match passthrough {
                Passthrough::None => 0,
                Passthrough::Tmux => 3,
            };
            assert_eq!(count(&transmit, b"\x1bPtmux;"), wrapped);

            let put = image.put_command(9, passthrough).into_bytes();
            let mut first = b"\r\n\n\n\n\x1b[4A".to_vec();
            first.extend(&transmit);
            first.extend(&put);
            first.extend(b"\x1b[4B");
            assert_eq!(frame_bytes(&image, 9, None, 4, passthrough), first);

            let mut next = transmit.clone();
            next.extend(b"\x1b[4A");
            next.extend(&put);
            next.extend(delete_image_command(8, true, passthrough).into_bytes());
            next.extend(b"\x1b[4B");
            assert_eq!(frame_bytes(&image, 9, Some(8), 4, passthrough), next);
        }
    }

    #[test]
    fn id_blocks_come_from_the_counter() {
        assert_eq!(id_block(0), 1);
        assert_eq!(id_block(254), 255);
        assert_eq!(id_block(255), 1);
        assert_eq!(id_block(u32::MAX), 1); // a multiple of 255
        // other tests take blocks too, but far fewer than 255 in between
        let (a, b) = (next_block(), next_block());
        assert!((1..=255).contains(&a) && (1..=255).contains(&b));
        assert_ne!(a, b);
    }

    #[test]
    fn frame_ids_are_non_zero_below_2_pow_24_and_rise() {
        for block in [1, 2, 128, 255] {
            for frame in [0, 1, 2, 65535, 65536, 65537, u32::MAX] {
                let id = frame_id(block, frame);
                assert!(id != 0 && id < 1 << 24, "block {block} frame {frame}: {id}");
                assert_eq!(id >> 16, block);
            }
            assert!((1..65535).all(|k| frame_id(block, k) < frame_id(block, k + 1)));
            // after 65535 frames they wrap
            assert!(frame_id(block, 65536) < frame_id(block, 65535));
            assert_eq!(frame_id(block, 65537), frame_id(block, 1));
        }
        assert_eq!(frame_id(255, 65535), (1 << 24) - 1);
    }

    #[test]
    fn a_placement_shows_the_image_then_replaces_it() {
        let terminal = Terminal::with_window(window());
        let image = rgb_image(30, 45); // 3 rows of 20 px
        let mut placement = terminal.place_in_buffer(&image).unwrap();
        assert_eq!(placement.size(), (30, 45));
        assert_eq!(placement.rows(), rows_covered(45, terminal.window()));
        assert_eq!(placement.rows(), 3);
        let first = frame_id(placement.block, 1);
        assert_eq!(
            placement.written(),
            frame_bytes(&image, first, None, 3, Passthrough::None)
        );

        placement.replace(&image).unwrap();
        placement.replace(&image).unwrap();
        let mut expected = frame_bytes(&image, first, None, 3, Passthrough::None);
        expected.extend(frame_bytes(
            &image,
            first + 1,
            Some(first),
            3,
            Passthrough::None,
        ));
        expected.extend(frame_bytes(
            &image,
            first + 2,
            Some(first + 1),
            3,
            Passthrough::None,
        ));
        assert_eq!(placement.written(), expected);
        assert_eq!(count(placement.written(), b"a=t,"), 3);
        assert_eq!(count(placement.written(), b"a=d,"), 2);
    }

    #[test]
    fn replace_rgb_sends_a_png() {
        let terminal = Terminal::with_window(window());
        let mut placement = terminal.place_in_buffer(&rgb_image(4, 2)).unwrap();
        placement.replace_rgb(&[255; 4 * 2 * 3], 4, 2).unwrap();
        let id = frame_id(placement.block, 2);
        let png = Image::png_from_rgb(&[255; 4 * 2 * 3], 4, 2).unwrap();
        assert!(placement.written().ends_with(&frame_bytes(
            &png,
            id,
            Some(id - 1),
            1,
            Passthrough::None
        )));
    }

    #[test]
    fn frame_ids_wrap_in_a_long_running_placement() {
        let terminal = Terminal::with_window(window());
        let image = rgb_image(1, 1);
        let mut placement = terminal.place_in_buffer(&image).unwrap();
        placement.frame = 65535;
        placement.replace(&image).unwrap();
        let block = placement.block << 16;
        assert!(placement.written().ends_with(&frame_bytes(
            &image,
            block,
            Some(block | 65535),
            1,
            Passthrough::None
        )));
    }

    #[test]
    fn a_replacement_of_another_size_is_an_error() {
        let terminal = Terminal::with_window(window());
        let mut placement = terminal.place_in_buffer(&rgb_image(30, 45)).unwrap();
        let written = placement.written().len();
        let err = placement.replace(&rgb_image(30, 46)).unwrap_err();
        assert!(matches!(
            err,
            Error::PlacementSize {
                expected: (30, 45),
                got: (30, 46)
            }
        ));
        assert_eq!(
            err.to_string(),
            "the new image is 30x46 pixels, but the image it replaces is 30x45; every frame \
             must have the same size"
        );
        // checked before the pixels are looked at
        let err = placement.replace_rgb(&[], 31, 45).unwrap_err();
        assert!(matches!(err, Error::PlacementSize { got: (31, 45), .. }));
        assert_eq!(placement.written().len(), written);
        assert_eq!(placement.frame, 1);
    }

    #[test]
    fn an_image_needs_a_line_to_spare_for_the_cursor() {
        // 50 rows of 20 px: 49 rows fit
        let terminal = Terminal::with_window(window());
        assert_eq!(
            terminal.place_in_buffer(&rgb_image(1, 980)).unwrap().rows(),
            49
        );
        let err = terminal.place_in_buffer(&rgb_image(1, 981)).unwrap_err();
        assert!(matches!(
            err,
            Error::ImageTooTall {
                rows: 50,
                screen_rows: 50
            }
        ));
        assert!(err.to_string().contains("50 lines tall"), "{err}");
    }

    #[test]
    fn write_errors_are_io_errors_and_keep_the_frame() {
        let terminal = Terminal::with_window(window());
        let image = rgb_image(1, 1);
        let err = Placement::new(terminal, &image, Sink::Closed).unwrap_err();
        assert!(matches!(err, Error::Io(ref e) if e.kind() == io::ErrorKind::BrokenPipe));

        let mut placement = terminal.place_in_buffer(&image).unwrap();
        placement.sink = Sink::Closed;
        assert!(matches!(placement.replace(&image), Err(Error::Io(_))));
        // the next frame still deletes the image on screen
        assert_eq!(placement.frame, 1);
    }

    #[test]
    fn placements_are_send_and_sync() {
        fn check<T: Send + Sync + 'static>() {}
        check::<Placement>();
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
