pub trait CtrlSeq {
    fn get_ctrl_seq(&self) -> String;
}

/// How image data reaches the terminal.
///
/// Only `Direct` works when the terminal runs on another machine (e.g. over SSH); the other media
/// name a file or shared-memory object that the terminal itself must be able to open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transmission {
    /// The data is sent inline in the escape sequence.
    Direct(Vec<u8>),
    /// A file the terminal reads (made absolute by `Image::new`).
    File(String),
    /// A temporary file the terminal reads and then deletes. Kitty only deletes files in a known
    /// temporary directory whose name contains `tty-graphics-protocol`.
    TempFile(String),
    /// A POSIX shared-memory object name.
    SharedMemory(String),
}

impl CtrlSeq for Transmission {
    fn get_ctrl_seq(&self) -> String {
        match self {
            Transmission::Direct(_) => String::from("t=d"),
            Transmission::File(_) => String::from("t=f"),
            Transmission::TempFile(_) => String::from("t=t"),
            Transmission::SharedMemory(_) => String::from("t=s"),
        }
    }
}

/// The format of the image data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// PNG data; the terminal reads the size from it.
    Png,
    /// PNG data scaled to fit a number of terminal rows and columns.
    PngBounded {
        /// Rows of text the image covers.
        rows: u32,
        /// Columns of text the image covers.
        cols: u32,
    },
    /// Raw 8-bit RGB pixels, row-major from the top row.
    Rgb {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
    /// Raw 8-bit RGBA pixels, row-major from the top row.
    Rgba {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
}

impl CtrlSeq for PixelFormat {
    fn get_ctrl_seq(&self) -> String {
        match self {
            PixelFormat::Png => String::from("f=100"),
            PixelFormat::PngBounded { cols, rows } => format!("f=100,c={cols},r={rows}"),
            PixelFormat::Rgb {
                width: pix_width,
                height: pix_height,
            } => format!("f=24,s={pix_width},v={pix_height}"),
            PixelFormat::Rgba {
                width: pix_width,
                height: pix_height,
            } => format!("f=32,s={pix_width},v={pix_height}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    TransmitDisplay,
    Query,
}

impl CtrlSeq for Action {
    fn get_ctrl_seq(&self) -> String {
        match self {
            Action::TransmitDisplay => String::from("a=T"),
            Action::Query => String::from("a=q"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metadata {
    Id(u32),
    MoreData(bool),
    /// Suppresses the terminal's replies: `Quiet(1)` suppresses `OK` replies, `Quiet(2)` also
    /// suppresses errors. Replies nobody reads would otherwise show up as typed input.
    Quiet(u8),
    /// Leaves the cursor where it is instead of moving it past the image.
    NoCursorMovement,
}

impl CtrlSeq for Metadata {
    fn get_ctrl_seq(&self) -> String {
        match self {
            Metadata::Id(id) => format!("i={id}"),
            Metadata::MoreData(more) => format!("m={}", if *more { 1 } else { 0 }),
            Metadata::Quiet(level) => format!("q={level}"),
            Metadata::NoCursorMovement => String::from("C=1"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Positioning {
    WithCellOffset { offset_x: u32, offset_y: u32 },
}

impl CtrlSeq for Positioning {
    fn get_ctrl_seq(&self) -> String {
        match self {
            Positioning::WithCellOffset { offset_x, offset_y } => {
                format!("X={offset_x},Y={offset_y}")
            }
        }
    }
}
