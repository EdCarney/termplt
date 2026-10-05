//! Subplots: one figure with a wide plot across the top and two plots below it. With a path
//! argument the figure is saved as an 800x600 PNG; without one it is shown in the terminal.
//!
//! ```bash
//! cargo run --example subplots
//! cargo run --release --example subplots -- subplots.png
//! ```

use termplt::prelude::*;

fn main() -> termplt::Result<()> {
    let xs: Vec<f64> = (0..200).map(|i| f64::from(i) * 0.05).collect();
    let sin: Vec<(f64, f64)> = xs.iter().map(|&x| (x, x.sin())).collect();
    let cos: Vec<(f64, f64)> = xs.iter().map(|&x| (x, x.cos())).collect();
    let spiral: Vec<(f64, f64)> = xs.iter().map(|&x| (x * x.cos(), x * x.sin())).collect();
    let squares: Vec<(f64, f64)> = xs.iter().map(|&x| (x, x * x)).collect();

    let fig = Figure::new(2, 2)
        .plot(
            0,
            ..,
            Plot::new()
                .line(Series::from(sin).with_label("sin"))
                .line(Series::from(cos).with_label("cos"))
                .title("Signals"),
        )
        .plot(1, 0, Plot::new().scatter(spiral).title("Spiral"))
        .plot(1, 1, Plot::new().line_points(squares).title("Squares"));

    match std::env::args().nth(1) {
        Some(path) => fig.size(800, 600).save_png(path),
        None => fig.show(),
    }
}
