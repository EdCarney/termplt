use crate::{
    common::Result,
    kitty_graphics::ctrl_seq::*,
    terminal_commands::{csi_cmds, kitty_cmds::KittyCommand, responses::TermCommand},
};
use image::{self, ImageFormat, ImageReader};
use std::{error::Error, fmt, io::Cursor, path::Path};

#[derive(Debug)]
pub enum ImageError {
    PositioningOutsideTerminalWindow,
    DisplayRegionExceedsImageBounds,
    KittyFormatUnsupported,
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for ImageError {}

pub enum PositioningType {
    ExactPixel { x: u32, y: u32 },
    Centered,
}

struct PositionDetails {
    row: u32,
    col: u32,
    offset_x: u32,
    offset_y: u32,
}

pub struct Image {
    format: PixelFormat,
    transmission: Transmission,
    width_pix: u32,
    height_pix: u32,
}

impl Image {
    pub fn new(format: PixelFormat, transmission: Transmission) -> Result<Image> {
        let (width_pix, height_pix) = match format {
            PixelFormat::Png => match transmission {
                Transmission::File(ref file_path) => {
                    image::image_dimensions(Path::new(&file_path))?
                }
                Transmission::Direct(ref bytes) => {
                    let cursor = Cursor::new(bytes);
                    ImageReader::with_format(cursor, ImageFormat::Png).into_dimensions()?
                }
                _ => panic!("Unsupported format"),
            },
            PixelFormat::PngBounded { cols, rows } => {
                let window_sz = termplt::get_window_size()?;
                let width = cols * window_sz.pix_per_col;
                let height = rows * window_sz.pix_per_row;
                (width, height)
            }
            PixelFormat::Rgb { width, height } => (width, height),
            PixelFormat::Rgba { width, height } => (width, height),
        };
        Ok(Image {
            format,
            transmission,
            width_pix,
            height_pix,
        })
    }

    pub fn display(&self) -> Result<()> {
        let attributes = vec![
            Action::TransmitDisplay.get_ctrl_seq(),
            self.format.get_ctrl_seq(),
            self.transmission.get_ctrl_seq(),
        ];
        self.display_with_attributes(&attributes)
    }

    pub fn display_at_position(&self, positioning: PositioningType) -> Result<()> {
        let window_sz = termplt::get_window_size()?;
        match positioning {
            PositioningType::ExactPixel { x, y } => {
                let row = (y / window_sz.pix_per_row) + 1;
                let col = (x / window_sz.pix_per_col) + 1;
                let offset_x = x % window_sz.pix_per_col;
                let offset_y = y % window_sz.pix_per_row;

                let attributes = vec![
                    self.format.get_ctrl_seq(),
                    self.transmission.get_ctrl_seq(),
                    Positioning::WithCellOffset { offset_x, offset_y }.get_ctrl_seq(),
                    Action::TransmitDisplay.get_ctrl_seq(),
                ];

                // move cursor, write data, then move back to original position
                let cursor_pos = csi_cmds::get_cursor_pos()?;
                csi_cmds::set_cursor_pos(row, col)?;
                self.display_with_attributes(&attributes)?;
                csi_cmds::set_cursor_pos(cursor_pos.row, cursor_pos.col)
            }
            PositioningType::Centered => {
                let x = (window_sz.x_pix / 2) - (self.width_pix / 2);
                let y = (window_sz.y_pix / 2) - (self.height_pix / 2);
                self.display_at_position(PositioningType::ExactPixel { x, y })
            }
        }
    }

    fn display_with_attributes(&self, attributes: &[String]) -> Result<()> {
        let cmd = match self.transmission {
            Transmission::Direct(ref bytes) => KittyCommand::new(bytes, &attributes),
            Transmission::File(ref file_path) => {
                KittyCommand::new(file_path.as_bytes(), &attributes)
            }
            _ => panic!("Unsupported type!"),
        };
        cmd.execute()
    }

    fn get_positioning_details(&self, x_pix: u32, y_pix: u32) -> Result<PositionDetails> {
        let window_sz = termplt::get_window_size()?;

        // check positioning specification is valid
        if x_pix > window_sz.x_pix || y_pix > window_sz.y_pix {
            Err(Box::new(ImageError::PositioningOutsideTerminalWindow))
        } else {
            let row = (y_pix / window_sz.pix_per_row) + 1;
            let col = (x_pix / window_sz.pix_per_col) + 1;
            let offset_x = x_pix % window_sz.pix_per_col;
            let offset_y = y_pix % window_sz.pix_per_row;
            Ok(PositionDetails {
                row,
                col,
                offset_x,
                offset_y,
            })
        }
    }
}
