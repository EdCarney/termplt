use termplt::prelude::*;

fn main() -> termplt::Result<()> {
    let terminal = Terminal::connect()?;
    let side = terminal.max_square_size();
    let mut canvas = Canvas::new(side, side, colors::BLACK);
    let mut placement = terminal.place(&Image::png_from_rgb(canvas.as_bytes(), side, side)?)?;
    for i in 0..side {
        canvas.put(i, i, colors::RED);
        placement.replace_rgb(canvas.as_bytes(), canvas.width(), canvas.height())?;
    }
    Ok(())
}
