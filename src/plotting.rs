/// Axes drawn around the plot area, with tick labels.
pub mod axes;
/// The canvas a graph is drawn on, and its layout.
pub mod canvas;
pub mod colors;
mod common;
/// Fonts for text on plots.
pub mod font;
/// A set of series on shared axes.
pub mod graph;
/// Grid lines at the tick positions.
pub mod grid_lines;
/// Axis-aligned rectangles.
pub mod limits;
/// Line styles.
pub mod line;
mod line_positioning;
/// Marker styles.
pub mod marker;
mod numbers;
/// 2D points.
pub mod point;
/// Data series.
pub mod series;
/// Bitmap text for labels.
pub mod text;
mod ticks;

pub use common::{Drawable, Graphable, MaskPoints, ToF64};
