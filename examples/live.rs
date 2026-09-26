//! A live plot: a sine wave sampled every 50 ms for 10 s, redrawn in place with the last 100
//! samples (a 5 s sliding window).
//!
//! ```bash
//! cargo run --example live
//! ```

use std::{
    thread,
    time::{Duration, Instant},
};
use termplt::prelude::*;

const INTERVAL: Duration = Duration::from_millis(50);
const SAMPLES: u32 = 200;
const WINDOW: usize = 100;

fn main() -> termplt::Result<()> {
    let mut plot = Plot::new()
        .line(Series::from(vec![(0.0, 0.0)]).with_label("sin(2t)"))
        .y_limits(-1.2, 1.2) // fixed, so only the x axis moves
        .title("Live plot")
        .x_label("t (s)");
    let mut live = plot.show_live()?;

    let start = Instant::now();
    for i in 1..=SAMPLES {
        let t = f64::from(i) * INTERVAL.as_secs_f64();
        let sine = &mut plot.series_mut()[0];
        sine.push(t, (2.0 * t).sin());
        sine.keep_last(WINDOW);
        live.update(&plot)?;
        // keep to the schedule however long a frame took to draw
        if let Some(wait) = (start + INTERVAL * i).checked_duration_since(Instant::now()) {
            thread::sleep(wait);
        }
    }
    Ok(())
}
