//! The crate's error type.

use crate::terminal_commands::responses::TerminalCommandError;
use std::{fmt, io};

/// A `Result` whose error is [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Everything that can go wrong while building, drawing or displaying a plot.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The graph has no finite data points (NaN and ±∞ are ignored).
    NoData,
    /// Every data point lies outside the explicit axis limits.
    NoVisibleData,
    /// Explicit limits are non-finite or inverted (`min > max`).
    InvalidLimits {
        /// `"x"` or `"y"`.
        axis: &'static str,
        /// The requested lower bound.
        min: f64,
        /// The requested upper bound.
        max: f64,
    },
    /// The data spans more than `f64` can represent, so it cannot be scaled.
    DataRangeTooLarge,
    /// The canvas has no room for the plot once the buffer, tick labels and marker insets are
    /// taken out (or the canvas has zero size).
    CanvasTooSmall {
        /// Width left for the plot area, in pixels.
        plot_width: u32,
        /// Height left for the plot area, in pixels.
        plot_height: u32,
    },
    /// Stdout is not a terminal (e.g. it is redirected to a file or pipe).
    NotATerminal,
    /// The terminal does not implement the Kitty graphics protocol.
    GraphicsUnsupported,
    /// The terminal implements the graphics protocol but rejected a test image.
    GraphicsRejected(String),
    /// Talking to the terminal failed (no terminal, no reply, or an unexpected reply).
    Terminal(TerminalCommandError),
    /// Running inside tmux with `allow-passthrough` off, so tmux would drop the image.
    TmuxPassthroughDisabled,
    /// The terminal size could not be determined.
    WindowSize(Box<Error>),
    /// The terminal reported a size of zero, or fewer pixels than cells.
    InvalidWindowSize {
        /// Reported rows.
        rows: u32,
        /// Reported columns.
        cols: u32,
    },
    /// An image position lies outside the terminal window.
    PositionOutsideWindow,
    /// The pixel format cannot be combined with the transmission medium.
    UnsupportedTransmission,
    /// A file path that cannot be sent to the terminal (it must be valid UTF-8).
    InvalidPath(String),
    /// Font data that is not a TrueType or OpenType font.
    InvalidFont(String),
    /// Encoding or decoding an image failed.
    Image(image::ImageError),
    /// An I/O error, e.g. writing to the terminal or a file.
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoData => write!(
                f,
                "the graph has no data points to plot (non-finite values are ignored)"
            ),
            Error::NoVisibleData => {
                write!(f, "no data points lie within the specified axis limits")
            }
            Error::InvalidLimits { axis, min, max } => {
                if !min.is_finite() || !max.is_finite() {
                    write!(f, "{axis} limits must be finite, got {min}..{max}")
                } else {
                    write!(
                        f,
                        "{axis} limits are inverted: min ({min}) is greater than max ({max})"
                    )
                }
            }
            Error::DataRangeTooLarge => write!(
                f,
                "the data range is too large to plot (exceeds the range of f64)"
            ),
            Error::CanvasTooSmall {
                plot_width,
                plot_height,
            } => write!(
                f,
                "the canvas is too small: the plot area would be {plot_width}x{plot_height} \
                 pixels after the buffer, labels and markers; use a larger canvas (or terminal \
                 window) or a smaller buffer or markers"
            ),
            Error::NotATerminal => write!(
                f,
                "stdout is not a terminal; plots are drawn with the Kitty graphics protocol and \
                 must be written to a terminal that supports it (e.g. Kitty, WezTerm, Ghostty)"
            ),
            Error::TmuxPassthroughDisabled => write!(
                f,
                "tmux is blocking the image: enable passthrough with `tmux set -g \
                 allow-passthrough on` (add `set -g allow-passthrough on` to ~/.tmux.conf to keep \
                 it)"
            ),
            Error::GraphicsUnsupported => write!(
                f,
                "this terminal does not support the Kitty graphics protocol (e.g. Kitty, \
                 WezTerm, Ghostty or Konsole)"
            ),
            Error::GraphicsRejected(msg) => {
                write!(f, "the terminal rejected a test image: {msg}")
            }
            Error::Terminal(e) => e.fmt(f),
            Error::WindowSize(e) => write!(f, "could not determine the terminal size: {e}"),
            Error::InvalidWindowSize { rows, cols } => write!(
                f,
                "the terminal reported invalid dimensions (rows={rows}, cols={cols})"
            ),
            Error::PositionOutsideWindow => {
                write!(f, "the image position lies outside the terminal window")
            }
            Error::UnsupportedTransmission => write!(
                f,
                "unsupported combination of pixel format and transmission medium"
            ),
            Error::InvalidPath(path) => write!(f, "image path is not valid UTF-8: {path}"),
            Error::InvalidFont(msg) => write!(f, "invalid font: {msg}"),
            Error::Image(e) => write!(f, "image error: {e}"),
            Error::Io(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    /// The wrapped error's own source. The wrapped error's message is already part of this
    /// error's message, so returning the wrapped error itself would make error reporters
    /// (`anyhow`'s `{:#}`, `eyre`) print it twice.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Terminal(e) => e.source(),
            Error::WindowSize(e) => e.source(),
            Error::Image(e) => e.source(),
            Error::Io(e) => e.source(),
            _ => None,
        }
    }
}

impl From<TerminalCommandError> for Error {
    fn from(e: TerminalCommandError) -> Error {
        Error::Terminal(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<image::ImageError> for Error {
    fn from(e: image::ImageError) -> Error {
        Error::Image(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_limits_messages() {
        let inverted = Error::InvalidLimits {
            axis: "x",
            min: 2.0,
            max: 1.0,
        };
        assert!(inverted.to_string().contains("inverted"));
        let infinite = Error::InvalidLimits {
            axis: "y",
            min: 0.0,
            max: f64::INFINITY,
        };
        assert!(infinite.to_string().contains("must be finite"));
    }

    #[test]
    fn error_is_send_sync_static() {
        fn check<T: Send + Sync + 'static>() {}
        check::<Error>();
    }

    #[test]
    fn messages_are_not_repeated_down_the_source_chain() {
        use std::error::Error as _;
        let inner = io::Error::other("closed");
        let e = Error::WindowSize(Box::new(Error::Io(io::Error::new(
            io::ErrorKind::BrokenPipe,
            inner,
        ))));
        // the message has the whole story once...
        assert!(e.to_string().contains("closed"));
        // ...and no source repeats a message that is already shown
        let mut shown = e.to_string();
        let mut source = e.source();
        while let Some(s) = source {
            assert!(!shown.contains(&s.to_string()), "{s} repeated in {shown}");
            shown.push_str(&s.to_string());
            source = s.source();
        }
    }
}
