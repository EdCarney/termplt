//! A live figure with two panels: a noisy signal on top and its running mean below, sampled every
//! 50 ms for 10 s and redrawn in place with the last 100 samples.
//!
//! ```bash
//! cargo run --example subplots_live
//! ```

use std::{
    thread,
    time::{Duration, Instant},
};
use termplt::prelude::*;

const INTERVAL: Duration = Duration::from_millis(50);
const SAMPLES: u32 = 200;
const WINDOW: usize = 100;

/// A deterministic stand-in for noise: a few incommensurate sines.
fn noise(t: f64) -> f64 {
    0.3 * (17.0 * t).sin() + 0.2 * (41.0 * t + 1.0).sin() + 0.1 * (97.0 * t + 2.0).sin()
}

fn empty(label: &str) -> Plot {
    Plot::new().line(Series::from(Vec::<(f64, f64)>::new()).with_label(label))
}

fn main() -> termplt::Result<()> {
    let mut fig = Figure::new(2, 1)
        .plot(0, 0, empty("signal").title("Signal").y_limits(-2.0, 2.0))
        .plot(
            1,
            0,
            empty("mean")
                .title("Running mean")
                .x_label("t (s)")
                .y_limits(-1.0, 1.0),
        );
    let mut live = fig.show_live()?;

    let (mut sum, start) = (0.0, Instant::now());
    for i in 1..=SAMPLES {
        let t = f64::from(i) * INTERVAL.as_secs_f64();
        let value = (2.0 * t).sin() + noise(t);
        sum += value;
        let mean = sum / f64::from(i);
        let [top, bottom] = fig.plots_mut() else {
            unreachable!("the figure has two plots")
        };
        for (plot, y) in [(top, value), (bottom, mean)] {
            let series = &mut plot.series_mut()[0];
            series.push(t, y);
            series.keep_last(WINDOW);
        }
        live.update(&fig)?;
        // keep to the schedule however long a frame took to draw
        if let Some(wait) = (start + INTERVAL * i).checked_duration_since(Instant::now()) {
            thread::sleep(wait);
        }
    }
    Ok(())
}
