//! The types most plots need: `use termplt::prelude::*;`.

pub use crate::{
    Error, Plot, Result,
    plotting::{
        axes::{Axes, AxesPositioning},
        canvas::{BufferType, TerminalCanvas},
        colors,
        font::Font,
        graph::Graph,
        grid_lines::GridLines,
        legend::LegendLocation,
        line::LineStyle,
        marker::MarkerStyle,
        point::Point,
        series::Series,
        text::TextStyle,
    },
    terminal::Terminal,
};
