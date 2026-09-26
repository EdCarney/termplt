//! Follow mode (`--follow`): stdin read as lines arrive, and frames drawn at most once per
//! interval.

use crate::{
    Result,
    data::{Column, Columns, TableReader},
};
use std::{
    io::{self, BufRead},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};
use termplt::plotting::point::Point;

/// The frame interval without `--interval`.
pub const DEFAULT_INTERVAL: Duration = Duration::from_millis(100);

/// How stdin is named in messages.
const SOURCE: &str = "stdin";

/// Reads stdin on a thread and sends each line (without its terminator) as it arrives. The
/// channel closes at the end of the input, after a read error.
pub fn read_lines() -> Receiver<io::Result<String>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            let failed = line.is_err();
            if sender.send(line).is_err() || failed {
                break;
            }
        }
    });
    receiver
}

/// The series read from stdin, which share one table: one reader finds the header and splits
/// the lines, and each series takes its columns from the rows.
#[derive(Debug)]
pub struct Stream {
    reader: TableReader,
    series: Vec<Streamed>,
    /// Data rows read so far; the x value of `Column::RowNumber` is the row's number from 0.
    rows: usize,
    /// Rows skipped because a series found a missing value in them.
    missing_rows: usize,
    /// Points skipped for NaN or infinite coordinates.
    non_finite: usize,
}

#[derive(Debug)]
struct Streamed {
    /// The series' index among all series.
    index: usize,
    x: Option<Column>,
    y: Option<Column>,
    /// Resolved once the header, or the first data row of a file without one, is known.
    columns: Option<Columns>,
}

impl Stream {
    /// A stream for the series at `index` with the requested columns.
    pub fn new(
        series: impl IntoIterator<Item = (usize, Option<Column>, Option<Column>)>,
    ) -> Stream {
        Stream {
            reader: TableReader::new(),
            series: (series.into_iter())
                .map(|(index, x, y)| Streamed {
                    index,
                    x,
                    y,
                    columns: None,
                })
                .collect(),
            rows: 0,
            missing_rows: 0,
            non_finite: 0,
        }
    }

    /// Reads one line and returns its points as (series index, point). Rows with a missing
    /// value and points with NaN or infinite coordinates are skipped and counted; a row that
    /// cannot be read is an error naming its line.
    pub fn push_line(&mut self, line: &str) -> Result<Vec<(usize, Point<f64>)>> {
        let row = self.reader.push_line(line);
        if let Some(width) = self.reader.width() {
            resolve(&mut self.series, self.reader.header(), width)?;
        }
        let Some(row) = row else {
            return Ok(Vec::new());
        };
        let row_number = self.rows;
        self.rows += 1;
        let mut points = Vec::new();
        let mut missing = false;
        for series in &self.series {
            let columns = (series.columns.as_ref()).expect("resolved by the first data row");
            match columns.point(row_number, &row, SOURCE)? {
                None => missing = true,
                Some(p) if p.x.is_finite() && p.y.is_finite() => points.push((series.index, p)),
                Some(_) => self.non_finite += 1,
            }
        }
        self.missing_rows += usize::from(missing);
        Ok(points)
    }

    /// Ends the input. A stream that never had a data row resolves its columns as an empty
    /// table does, so it fails the same way (a column name without a header).
    pub fn finish(&mut self) -> Result<()> {
        resolve(&mut self.series, None, 0)
    }

    /// The resolved columns of the series at `index`, if it is read from the stream.
    pub fn columns(&self, index: usize) -> Option<&Columns> {
        (self.series.iter())
            .find(|s| s.index == index)
            .and_then(|s| s.columns.as_ref())
    }

    /// The warning about the rows and points skipped so far, if any.
    pub fn skipped(&self) -> Option<String> {
        skipped_summary(self.missing_rows, self.non_finite)
    }
}

/// Resolves the columns of the series that have none yet.
fn resolve(series: &mut [Streamed], header: Option<&[String]>, width: usize) -> Result<()> {
    for series in series.iter_mut().filter(|s| s.columns.is_none()) {
        let columns =
            Columns::resolve(header, width, series.x.as_ref(), series.y.as_ref(), SOURCE)?;
        series.columns = Some(columns);
    }
    Ok(())
}

/// The warning for skipped rows and points, worded like the ones printed for whole files.
fn skipped_summary(missing_rows: usize, non_finite: usize) -> Option<String> {
    let missing = (missing_rows > 0).then(|| format!("{missing_rows} row(s) with missing values"));
    let non_finite =
        (non_finite > 0).then(|| format!("{non_finite} point(s) with NaN or infinite values"));
    let skipped = match (missing, non_finite) {
        (None, None) => return None,
        (Some(one), None) | (None, Some(one)) => one,
        (Some(missing), Some(non_finite)) => format!("{missing} and {non_finite}"),
    };
    Some(format!("warning: skipped {skipped} in {SOURCE}"))
}

/// When frames are drawn: as soon as new points have arrived, but at most once per interval.
#[derive(Debug, Clone, Copy)]
pub struct Schedule {
    interval: Duration,
    last_frame: Option<Instant>,
    /// Points arrived since the last frame.
    pending: bool,
}

impl Schedule {
    pub fn new(interval: Duration) -> Schedule {
        Schedule {
            interval,
            last_frame: None,
            pending: false,
        }
    }

    /// Notes that points arrived.
    pub fn points_arrived(&mut self) {
        self.pending = true;
    }

    /// Whether a frame is due at `now`: points arrived since the last frame, and the interval
    /// has passed since it. The first frame is due as soon as there is a point.
    pub fn due(&self, now: Instant) -> bool {
        self.pending
            && (self.last_frame).is_none_or(|last| now.duration_since(last) >= self.interval)
    }

    /// How long to wait at `now` for more input before a frame is due; `None` when no frame
    /// is waiting, so only new input matters.
    pub fn wait(&self, now: Instant) -> Option<Duration> {
        let last = match (self.pending, self.last_frame) {
            (false, _) => return None,
            (true, None) => return Some(Duration::ZERO),
            (true, Some(last)) => last,
        };
        Some(self.interval.saturating_sub(now.duration_since(last)))
    }

    /// Whether points arrived since the last frame.
    pub fn pending(&self) -> bool {
        self.pending
    }

    /// Notes that a frame was drawn at `now`.
    pub fn drawn(&mut self, now: Instant) {
        self.pending = false;
        self.last_frame = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(c: &str) -> Option<Column> {
        Some(Column::parse(c).unwrap())
    }

    fn pts(points: &[(usize, f64, f64)]) -> Vec<(usize, Point<f64>)> {
        (points.iter())
            .map(|&(i, x, y)| (i, Point::new(x, y)))
            .collect()
    }

    /// Feeds every line, returning all the points.
    fn feed(stream: &mut Stream, lines: &[&str]) -> Result<Vec<(usize, Point<f64>)>> {
        let mut points = Vec::new();
        for line in lines {
            points.extend(stream.push_line(line)?);
        }
        Ok(points)
    }

    #[test]
    fn one_reader_feeds_every_series() {
        let mut stream = Stream::new([(1, column("time"), column("temp")), (3, None, column("3"))]);
        assert!(stream.columns(1).is_none());
        let points = feed(&mut stream, &["time,temp,rh", "0,20,50", "1,21,55"]).unwrap();
        assert_eq!(
            points,
            pts(&[
                (1, 0.0, 20.0),
                (3, 0.0, 50.0),
                (1, 1.0, 21.0),
                (3, 1.0, 55.0)
            ])
        );
        let columns = stream.columns(1).unwrap();
        assert_eq!(columns.names.y.as_deref(), Some("temp"));
        assert_eq!(stream.columns(3).unwrap().y_column, 3);
        assert!(stream.columns(0).is_none());
    }

    #[test]
    fn columns_are_resolved_with_the_header() {
        let mut stream = Stream::new([(0, None, None)]);
        // comments and blank lines come first; the header is known before any point
        assert!(feed(&mut stream, &["# sensors", ""]).unwrap().is_empty());
        assert!(stream.columns(0).is_none());
        assert!(feed(&mut stream, &["a,b"]).unwrap().is_empty());
        assert_eq!(stream.columns(0).unwrap().names.y.as_deref(), Some("b"));
    }

    #[test]
    fn a_single_column_is_plotted_against_the_row_number() {
        let mut stream = Stream::new([(0, None, None)]);
        let points = feed(&mut stream, &["5", "7", "", "9"]).unwrap();
        assert_eq!(points, pts(&[(0, 0.0, 5.0), (0, 1.0, 7.0), (0, 2.0, 9.0)]));
    }

    #[test]
    fn the_row_number_counts_skipped_rows() {
        let mut stream = Stream::new([(0, column("index"), column("2"))]);
        let points = feed(&mut stream, &["x,y", "1,10", "2,NA", "3,30"]).unwrap();
        assert_eq!(points, pts(&[(0, 0.0, 10.0), (0, 2.0, 30.0)]));
    }

    #[test]
    fn a_column_name_without_a_header_is_an_error_at_the_first_row() {
        let mut stream = Stream::new([(0, None, column("temp"))]);
        let err = stream.push_line("1,2").unwrap_err().to_string();
        assert_eq!(
            err,
            "stdin has no header row, so column 'temp' cannot be found; use a 1-based column \
             number instead"
        );
    }

    #[test]
    fn a_bad_row_is_an_error_naming_its_line() {
        let mut stream = Stream::new([(0, None, None)]);
        let err = feed(&mut stream, &["x,y", "1,2", "3,foo"])
            .unwrap_err()
            .to_string();
        assert_eq!(
            err,
            "stdin:3: cannot parse y value 'foo' (column 'y') as a number"
        );
    }

    #[test]
    fn skipped_rows_and_points_are_counted() {
        let mut stream = Stream::new([(0, None, column("2")), (1, None, column("3"))]);
        let points = feed(
            &mut stream,
            &["1,2,3", "2,NA,3", "3,NA,NA", "4,nan,4", "5,inf,-inf"],
        )
        .unwrap();
        assert_eq!(
            points,
            pts(&[(0, 1.0, 2.0), (1, 1.0, 3.0), (1, 2.0, 3.0), (1, 4.0, 4.0)])
        );
        // a row missing a value for one series or for both is one row
        assert_eq!(stream.missing_rows, 2);
        assert_eq!(stream.non_finite, 3);
        assert_eq!(
            stream.skipped().unwrap(),
            "warning: skipped 2 row(s) with missing values and 3 point(s) with NaN or infinite \
             values in stdin"
        );
    }

    #[test]
    fn a_stream_without_rows_fails_like_an_empty_table() {
        let mut stream = Stream::new([(0, None, None)]);
        feed(&mut stream, &["# nothing"]).unwrap();
        stream.finish().unwrap();
        assert_eq!(stream.columns(0).unwrap().y_column, 2);

        let mut stream = Stream::new([(0, None, column("temp"))]);
        let err = stream.finish().unwrap_err().to_string();
        assert!(err.starts_with("stdin has no header row"), "{err}");
        // columns resolved from the data stay
        let mut stream = Stream::new([(0, None, column("1"))]);
        feed(&mut stream, &["7"]).unwrap();
        stream.finish().unwrap();
        assert_eq!(stream.columns(0).unwrap().y_column, 1);
    }

    #[test]
    fn the_summary_names_what_was_skipped() {
        assert_eq!(skipped_summary(0, 0), None);
        assert_eq!(
            skipped_summary(3, 0).unwrap(),
            "warning: skipped 3 row(s) with missing values in stdin"
        );
        assert_eq!(
            skipped_summary(0, 1).unwrap(),
            "warning: skipped 1 point(s) with NaN or infinite values in stdin"
        );
        assert_eq!(
            skipped_summary(3, 1).unwrap(),
            "warning: skipped 3 row(s) with missing values and 1 point(s) with NaN or infinite \
             values in stdin"
        );
        assert_eq!(
            skipped_summary(10, 20).unwrap(),
            "warning: skipped 10 row(s) with missing values and 20 point(s) with NaN or \
             infinite values in stdin"
        );
    }

    const MS: Duration = Duration::from_millis(1);

    #[test]
    fn nothing_is_due_without_new_points() {
        let now = Instant::now();
        let mut schedule = Schedule::new(100 * MS);
        assert!(!schedule.due(now));
        assert_eq!(schedule.wait(now), None);
        schedule.drawn(now);
        assert!(!schedule.due(now + 1000 * MS));
        assert_eq!(schedule.wait(now + 1000 * MS), None);
    }

    #[test]
    fn the_first_frame_is_due_with_the_first_point() {
        let now = Instant::now();
        let mut schedule = Schedule::new(100 * MS);
        schedule.points_arrived();
        assert!(schedule.due(now));
        assert_eq!(schedule.wait(now), Some(Duration::ZERO));
    }

    #[test]
    fn later_frames_wait_for_the_interval() {
        let start = Instant::now();
        let mut schedule = Schedule::new(100 * MS);
        schedule.points_arrived();
        schedule.drawn(start);
        assert!(!schedule.pending());

        schedule.points_arrived();
        assert!(schedule.pending());
        assert!(!schedule.due(start + 30 * MS));
        assert_eq!(schedule.wait(start + 30 * MS), Some(70 * MS));
        assert!(schedule.due(start + 100 * MS));
        assert_eq!(schedule.wait(start + 100 * MS), Some(Duration::ZERO));
        // late is due at once
        assert!(schedule.due(start + 250 * MS));
        assert_eq!(schedule.wait(start + 250 * MS), Some(Duration::ZERO));
    }

    #[test]
    fn a_zero_interval_draws_whenever_points_arrive() {
        let now = Instant::now();
        let mut schedule = Schedule::new(Duration::ZERO);
        schedule.points_arrived();
        schedule.drawn(now);
        schedule.points_arrived();
        assert!(schedule.due(now));
        assert_eq!(schedule.wait(now), Some(Duration::ZERO));
    }
}
