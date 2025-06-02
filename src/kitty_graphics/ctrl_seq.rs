pub trait CtrlSeq {
    fn get_ctrl_seq(&self) -> String;
}

pub enum Transmission {
    Direct,
    File,
    TempFile,
    SharedMemory,
}

impl CtrlSeq for Transmission {
    fn get_ctrl_seq(&self) -> String {
        match self {
            Transmission::Direct => String::from("t=d"),
            Transmission::File => String::from("t=f"),
            Transmission::TempFile => String::from("t=t"),
            Transmission::SharedMemory => String::from("t=s"),
        }
    }
}

pub enum PixelFormat {
    Png,
    PngBounded { cols: u32, rows: u32 },
    Rgb { width: u32, height: u32 },
    Rgba { width: u32, height: u32 },
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

pub enum Metadata {
    Id(u32),
    MoreData(bool),
    StackingOrder(u16),
}

impl CtrlSeq for Metadata {
    fn get_ctrl_seq(&self) -> String {
        match self {
            Metadata::Id(id) => format!("i={id}"),
            Metadata::MoreData(more) => format!("m={}", if *more { 1 } else { 0 }),
            Metadata::StackingOrder(z) => format!("z={z}"),
        }
    }
}

pub enum Positioning {
    Current,
    WithCellOffset { offset_x: u16, offset_y: u16 },
}

impl CtrlSeq for Positioning {
    fn get_ctrl_seq(&self) -> String {
        match self {
            Positioning::Current => String::from(""),
            Positioning::WithCellOffset { offset_x, offset_y } => {
                format!("X={offset_x},Y={offset_y}")
            }
        }
    }
}

pub enum DisplayRegion {
    Rectangle {
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    },
    Rows(u16),
    Cols(u16),
    RowsCols {
        rows: u16,
        cols: u16,
    },
}

impl CtrlSeq for DisplayRegion {
    fn get_ctrl_seq(&self) -> String {
        match self {
            DisplayRegion::Rectangle {
                x,
                y,
                width,
                height,
            } => format!("x={x},y={y},w={width},h={height}"),
            DisplayRegion::Rows(rows) => format!("r={rows}"),
            DisplayRegion::Cols(cols) => format!("c={cols}"),
            DisplayRegion::RowsCols { rows, cols } => format!("r={rows},c={cols}"),
        }
    }
}
