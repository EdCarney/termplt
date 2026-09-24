use crate::terminal_commands::csi_cmds;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowSize {
    pub rows: u32,
    pub cols: u32,
    pub x_pix: u32,
    pub y_pix: u32,
    pub pix_per_row: u32,
    pub pix_per_col: u32,
}

/// Returns the terminal's size in cells and pixels.
///
/// The operating system's record of the size (`TIOCGWINSZ` on Unix) is used when it is complete,
/// since reading it needs no round trip to the terminal. Missing parts (pixel sizes are often
/// zero, and Windows has no pixel sizes at all) are queried from the terminal with
/// `CSI 14 t` (pixels) and `CSI 18 t` (cells).
pub fn get_window_size() -> Result<WindowSize> {
    let (rows, cols, x_pix, y_pix) = combine(
        os_window_size(),
        csi_cmds::get_text_area_size_cells,
        csi_cmds::get_text_area_size_pixels,
    )
    .map_err(|e| Error::WindowSize(Box::new(e)))?;

    if rows == 0 || cols == 0 || x_pix == 0 || y_pix == 0 {
        return Err(Error::InvalidWindowSize { rows, cols });
    }

    let pix_per_col = x_pix / cols;
    let pix_per_row = y_pix / rows;

    Ok(WindowSize {
        rows,
        cols,
        x_pix,
        y_pix,
        pix_per_col,
        pix_per_row,
    })
}

/// The size the operating system reports: (rows, cols, x_pix, y_pix), with zeros where unknown.
fn os_window_size() -> Option<(u32, u32, u32, u32)> {
    match crossterm::terminal::window_size() {
        Ok(ws) => Some((
            ws.rows.into(),
            ws.columns.into(),
            ws.width.into(),
            ws.height.into(),
        )),
        // e.g. Windows, which only knows the size in cells
        Err(_) => crossterm::terminal::size()
            .ok()
            .map(|(cols, rows)| (rows.into(), cols.into(), 0, 0)),
    }
}

type Query = fn() -> Result<(u32, u32)>;

/// Fills in whatever the OS did not report by querying the terminal. `query_cells` returns
/// (rows, cols), `query_pixels` returns (width, height).
fn combine(
    os: Option<(u32, u32, u32, u32)>,
    query_cells: Query,
    query_pixels: Query,
) -> Result<(u32, u32, u32, u32)> {
    let (mut rows, mut cols, mut x_pix, mut y_pix) = os.unwrap_or_default();
    if rows == 0 || cols == 0 {
        (rows, cols) = query_cells()?;
    }
    if x_pix == 0 || y_pix == 0 {
        (x_pix, y_pix) = query_pixels()?;
    }
    Ok((rows, cols, x_pix, y_pix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells() -> Result<(u32, u32)> {
        Ok((50, 160))
    }

    fn pixels() -> Result<(u32, u32)> {
        Ok((1600, 1000))
    }

    fn unreachable_query() -> Result<(u32, u32)> {
        panic!("the terminal should not be queried")
    }

    #[test]
    fn complete_os_size_needs_no_queries() {
        let size = combine(
            Some((40, 100, 1000, 800)),
            unreachable_query,
            unreachable_query,
        );
        assert_eq!(size.unwrap(), (40, 100, 1000, 800));
    }

    #[test]
    fn missing_pixels_are_queried() {
        let size = combine(Some((40, 100, 0, 0)), unreachable_query, pixels);
        assert_eq!(size.unwrap(), (40, 100, 1600, 1000));
    }

    #[test]
    fn no_os_size_queries_everything() {
        assert_eq!(combine(None, cells, pixels).unwrap(), (50, 160, 1600, 1000));
    }

    #[test]
    fn query_errors_propagate() {
        fn failing() -> Result<(u32, u32)> {
            Err(Error::NoData)
        }
        assert!(combine(Some((40, 100, 0, 0)), cells, failing).is_err());
    }
}
