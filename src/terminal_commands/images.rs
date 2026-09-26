use crate::{
    Error,
    common::Result,
    kitty_graphics::ctrl_seq::*,
    terminal_commands::{
        csi_cmds,
        kitty_cmds::{KittyCommand, Passthrough},
        responses::TermCommand,
    },
    window_ctrl::{self, WindowSize},
};
use image::{
    self, ExtendedColorType, ImageEncoder, ImageFormat, ImageReader,
    codecs::png::{CompressionType, FilterType, PngEncoder},
};
use std::{io::Cursor, path::Path};

/// Where [`Image::display_at_position`] places an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositioningType {
    /// With the top-left corner at pixel (`x`, `y`) of the window.
    ExactPixel {
        /// Pixels from the left edge.
        x: u32,
        /// Pixels from the top edge.
        y: u32,
    },
    /// Centered in the window.
    Centered,
}

struct PositionDetails {
    row: u32,
    col: u32,
    offset_x: u32,
    offset_y: u32,
}

/// An image ready to be sent to the terminal with the Kitty graphics protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    format: PixelFormat,
    transmission: Transmission,
    width_pix: u32,
    height_pix: u32,
}

impl Image {
    /// Prepares an image; reads its size from PNG data or files, or from the terminal for
    /// [`PixelFormat::PngBounded`].
    pub fn new(format: PixelFormat, transmission: Transmission) -> Result<Image> {
        let transmission = absolute_paths(transmission)?;
        let (width_pix, height_pix) = match format {
            PixelFormat::Png => match transmission {
                Transmission::File(ref file_path) | Transmission::TempFile(ref file_path) => {
                    image::image_dimensions(Path::new(&file_path))?
                }
                Transmission::Direct(ref bytes) => {
                    let cursor = Cursor::new(bytes);
                    ImageReader::with_format(cursor, ImageFormat::Png).into_dimensions()?
                }
                // the size of a PNG in shared memory cannot be read without mapping it
                Transmission::SharedMemory(_) => {
                    return Err(Error::UnsupportedTransmission);
                }
            },
            PixelFormat::PngBounded { cols, rows } => {
                let window_sz = window_ctrl::get_window_size()?;
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

    /// Encodes RGB8 pixel data (`width * height * 3` bytes, row-major) as a PNG to be sent
    /// directly. Plots compress well (typically ~100x; a 1600x800 plot is about 30 KB instead of 3.8 MB), which matters over slow links such
    /// as SSH, where raw pixel data can take seconds to transmit.
    pub fn png_from_rgb(rgb: &[u8], width: u32, height: u32) -> Result<Image> {
        let mut png = Vec::new();
        PngEncoder::new_with_quality(&mut png, CompressionType::Default, FilterType::Adaptive)
            .write_image(rgb, width, height, ExtendedColorType::Rgb8)?;
        Image::new(PixelFormat::Png, Transmission::Direct(png))
    }

    /// Width in pixels.
    pub fn width(&self) -> u32 {
        self.width_pix
    }

    /// Height in pixels.
    pub fn height(&self) -> u32 {
        self.height_pix
    }

    /// The number of bytes of image data sent to the terminal (before base64 encoding), or the
    /// length of the path/name for other transmission media.
    pub fn payload_len(&self) -> usize {
        self.payload().len()
    }

    /// The payload of a transmission: the image data for direct transmission; for every other
    /// medium, the path or name of the object holding the image data.
    fn payload(&self) -> &[u8] {
        match &self.transmission {
            Transmission::Direct(bytes) => bytes,
            Transmission::File(name)
            | Transmission::TempFile(name)
            | Transmission::SharedMemory(name) => name.as_bytes(),
        }
    }

    /// The command that stores the image under `id` without displaying it (`a=t`), for
    /// [`Image::put_command`] to display later.
    // used by live plots (#51)
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn transmit_command(&self, id: u32, passthrough: Passthrough) -> KittyCommand {
        let attributes = [
            Action::Transmit.get_ctrl_seq(),
            self.format.get_ctrl_seq(),
            self.transmission.get_ctrl_seq(),
            Metadata::Id(id).get_ctrl_seq(),
            // nothing reads the replies
            Metadata::Quiet(2).get_ctrl_seq(),
        ];
        KittyCommand::with_passthrough(self.payload(), &attributes, passthrough)
    }

    /// The command that displays the image stored under `id` at the cursor (`a=p`), leaving the
    /// cursor where it is.
    // used by live plots (#51)
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn put_command(&self, id: u32, passthrough: Passthrough) -> KittyCommand {
        let mut ctrl = vec![Action::Put.get_ctrl_seq(), Metadata::Id(id).get_ctrl_seq()];
        // the cell bounds are display keys, which the transmit command doesn't keep
        if let PixelFormat::PngBounded { rows, cols } = self.format {
            ctrl.push(format!("c={cols},r={rows}"));
        }
        ctrl.push(Metadata::NoCursorMovement.get_ctrl_seq());
        ctrl.push(Metadata::Quiet(2).get_ctrl_seq());
        KittyCommand::control_only(&ctrl, passthrough)
    }

    /// Displays the image at the cursor. The terminal moves the cursor below the image.
    pub fn display(&self) -> Result<()> {
        self.display_with_attributes(&self.base_attributes())
    }

    /// Displays the image at the cursor and leaves the cursor where it was, e.g. to move it
    /// past the image with newlines inside a multiplexer that doesn't know the image is there.
    pub fn display_without_moving_cursor(&self) -> Result<()> {
        let mut attributes = self.base_attributes();
        attributes.push(Metadata::NoCursorMovement.get_ctrl_seq());
        self.display_with_attributes(&attributes)
    }

    fn base_attributes(&self) -> Vec<String> {
        vec![
            Action::TransmitDisplay.get_ctrl_seq(),
            self.format.get_ctrl_seq(),
            self.transmission.get_ctrl_seq(),
            // nothing reads replies to display commands
            Metadata::Quiet(2).get_ctrl_seq(),
        ]
    }

    /// Displays the image at a position in the window, then restores the cursor.
    pub fn display_at_position(&self, positioning: PositioningType) -> Result<()> {
        let window_sz = window_ctrl::get_window_size()?;
        match positioning {
            PositioningType::ExactPixel { x, y } => {
                let PositionDetails {
                    row,
                    col,
                    offset_x,
                    offset_y,
                } = Self::get_positioning_details(&window_sz, x, y)?;

                let mut attributes = self.base_attributes();
                attributes.push(Positioning::WithCellOffset { offset_x, offset_y }.get_ctrl_seq());

                // move cursor, write data, then move back to original position
                let cursor_pos = csi_cmds::get_cursor_pos()?;
                csi_cmds::set_cursor_pos(row, col)?;
                self.display_with_attributes(&attributes)?;
                csi_cmds::set_cursor_pos(cursor_pos.row, cursor_pos.col)
            }
            PositioningType::Centered => {
                // images larger than the window are anchored at the top-left corner
                let x = (window_sz.x_pix / 2).saturating_sub(self.width_pix / 2);
                let y = (window_sz.y_pix / 2).saturating_sub(self.height_pix / 2);
                self.display_at_position(PositioningType::ExactPixel { x, y })
            }
        }
    }

    fn display_with_attributes(&self, attributes: &[String]) -> Result<()> {
        KittyCommand::new(self.payload(), attributes).execute()
    }

    fn get_positioning_details(
        window_sz: &WindowSize,
        x_pix: u32,
        y_pix: u32,
    ) -> Result<PositionDetails> {
        // check positioning specification is valid
        if x_pix > window_sz.x_pix || y_pix > window_sz.y_pix {
            Err(Error::PositionOutsideWindow)
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

/// The command that deletes every placement of the image stored under `id` (`a=d`), and with
/// `free_data` also the stored image data.
// used by live plots (#51)
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn delete_image_command(
    id: u32,
    free_data: bool,
    passthrough: Passthrough,
) -> KittyCommand {
    let ctrl = [
        Action::Delete.get_ctrl_seq(),
        DeleteTarget::Image { id, free_data }.get_ctrl_seq(),
        // nothing reads the replies
        Metadata::Quiet(2).get_ctrl_seq(),
    ];
    KittyCommand::control_only(&ctrl, passthrough)
}

/// Kitty requires absolute paths for file transmission (relative paths would be resolved against
/// the terminal's working directory, not ours).
fn absolute_paths(transmission: Transmission) -> Result<Transmission> {
    let absolute = |name: String| -> Result<String> {
        let path = std::path::absolute(&name)?;
        path.to_str()
            .map(str::to_string)
            .ok_or_else(|| Error::InvalidPath(path.display().to_string()))
    };
    Ok(match transmission {
        Transmission::File(name) => Transmission::File(absolute(name)?),
        Transmission::TempFile(name) => Transmission::TempFile(absolute(name)?),
        other => other,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_paths_are_made_absolute() {
        let Transmission::File(path) =
            absolute_paths(Transmission::File(String::from("plot.png"))).unwrap()
        else {
            panic!("transmission type changed");
        };
        assert!(Path::new(&path).is_absolute());
        assert!(path.ends_with("plot.png"));
    }

    #[test]
    fn png_from_rgb_round_trips() {
        let (w, h) = (40u32, 30u32);
        let rgb: Vec<u8> = (0..w * h * 3).map(|i| (i % 251) as u8).collect();
        let image = Image::png_from_rgb(&rgb, w, h).unwrap();
        assert_eq!((image.width_pix, image.height_pix), (w, h));
        let Transmission::Direct(ref png) = image.transmission else {
            panic!("expected direct transmission");
        };
        let decoded = image::load_from_memory_with_format(png, ImageFormat::Png)
            .unwrap()
            .to_rgb8();
        assert_eq!(decoded.as_raw(), &rgb);
    }

    #[test]
    fn display_commands_suppress_replies() {
        let image = Image::new(
            PixelFormat::Rgb {
                width: 1,
                height: 1,
            },
            Transmission::Direct(vec![0, 0, 0]),
        )
        .unwrap();
        let attributes = image.base_attributes();
        assert!(attributes.contains(&String::from("q=2")));
        assert!(attributes.contains(&String::from("a=T")));
    }

    fn rgb_image(width: u32, height: u32) -> Image {
        Image::new(
            PixelFormat::Rgb { width, height },
            Transmission::Direct(vec![0; (width * height * 3) as usize]),
        )
        .unwrap()
    }

    /// Wraps one APC sequence in a tmux passthrough, doubling its escapes.
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

    #[test]
    fn transmit_command_stores_the_image_under_an_id() {
        let cmd = rgb_image(1, 1).transmit_command(7, Passthrough::None);
        assert_eq!(
            cmd.into_bytes(),
            b"\x1b_Ga=t,f=24,s=1,v=1,t=d,i=7,q=2,m=0;AAAA\x1b\\"
        );
    }

    #[test]
    fn transmit_command_sends_png_data_directly() {
        let image = Image::png_from_rgb(&[0, 0, 0], 1, 1).unwrap();
        let bytes = image.transmit_command(3, Passthrough::None).into_bytes();
        assert!(bytes.starts_with(b"\x1b_Ga=t,f=100,t=d,i=3,q=2,m=0;"));
        assert!(bytes.ends_with(b"\x1b\\"));
    }

    #[test]
    fn transmit_command_puts_the_keys_on_the_first_chunk_only() {
        // 3075 bytes encode to 4100 base64 characters: a full chunk of 4096, then 4
        let cmd = rgb_image(1025, 1).transmit_command(7, Passthrough::None);
        let mut expected = b"\x1b_Ga=t,f=24,s=1025,v=1,t=d,i=7,q=2,m=1;".to_vec();
        expected.extend_from_slice(&[b'A'; 4096]);
        expected.extend_from_slice(b"\x1b\\\x1b_Gm=0;AAAA\x1b\\");
        assert_eq!(cmd.into_bytes(), expected);
    }

    #[test]
    fn transmit_command_in_tmux_wraps_each_chunk() {
        let image = rgb_image(1025, 1);
        let plain = image.transmit_command(7, Passthrough::None).into_bytes();
        let tmux = image.transmit_command(7, Passthrough::Tmux).into_bytes();

        // neither the keys nor base64 contain a backslash, so each sequence ends at one
        let chunks: Vec<&[u8]> = plain.split_inclusive(|&b| b == b'\\').collect();
        assert_eq!(chunks.len(), 2);
        let expected: Vec<u8> = chunks.into_iter().flat_map(tmux_wrapped).collect();
        assert_eq!(tmux, expected);
    }

    #[test]
    fn transmit_command_sends_the_name_for_other_media() {
        let image = Image::new(
            PixelFormat::Rgb {
                width: 1,
                height: 1,
            },
            Transmission::SharedMemory(String::from("termplt-frame")),
        )
        .unwrap();
        assert_eq!(
            image.transmit_command(4, Passthrough::None).into_bytes(),
            b"\x1b_Ga=t,f=24,s=1,v=1,t=s,i=4,q=2,m=0;dGVybXBsdC1mcmFtZQ==\x1b\\"
        );
    }

    #[test]
    fn put_command_displays_a_stored_image_without_moving_the_cursor() {
        for image in [
            rgb_image(1, 1),
            Image::png_from_rgb(&[0, 0, 0], 1, 1).unwrap(),
        ] {
            assert_eq!(
                image.put_command(5, Passthrough::None).into_bytes(),
                b"\x1b_Ga=p,i=5,C=1,q=2\x1b\\"
            );
            assert_eq!(
                image.put_command(5, Passthrough::Tmux).into_bytes(),
                tmux_wrapped(b"\x1b_Ga=p,i=5,C=1,q=2\x1b\\")
            );
        }
    }

    #[test]
    fn put_command_keeps_the_cell_bounds() {
        // built directly: `Image::new` asks the terminal for its size for bounded PNGs
        let image = Image {
            format: PixelFormat::PngBounded { rows: 3, cols: 10 },
            transmission: Transmission::Direct(Vec::new()),
            width_pix: 90,
            height_pix: 54,
        };
        assert_eq!(
            image.put_command(5, Passthrough::None).into_bytes(),
            b"\x1b_Ga=p,i=5,c=10,r=3,C=1,q=2\x1b\\"
        );
    }

    #[test]
    fn delete_image_command_frees_the_data_on_request() {
        assert_eq!(
            delete_image_command(5, true, Passthrough::None).into_bytes(),
            b"\x1b_Ga=d,d=I,i=5,q=2\x1b\\"
        );
        assert_eq!(
            delete_image_command(5, false, Passthrough::None).into_bytes(),
            b"\x1b_Ga=d,d=i,i=5,q=2\x1b\\"
        );
        assert_eq!(
            delete_image_command(5, true, Passthrough::Tmux).into_bytes(),
            b"\x1bPtmux;\x1b\x1b_Ga=d,d=I,i=5,q=2\x1b\x1b\\\x1b\\"
        );
    }
}
