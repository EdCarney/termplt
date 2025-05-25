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
    Rgba,
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
            PixelFormat::Rgba => String::from("f=32"),
        }
    }
}

pub enum Action {
    TransmitDisplay,
}

impl CtrlSeq for Action {
    fn get_ctrl_seq(&self) -> String {
        match self {
            Action::TransmitDisplay => String::from("a=T"),
        }
    }
}

