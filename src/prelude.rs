//! The types most plots need: `use termplt::prelude::*;`.

pub use crate::{
    Error, Plot, Result,
    plotting::{
        axes::{Axes, AxesPositioning},
        canvas::{BufferType, TerminalCanvas},
        colors,
        graph::Graph,
        grid_lines::GridLines,
        line::LineStyle,
        marker::MarkerStyle,
        point::Point,
        series::Series,
        text::TextStyle,
    },
    terminal::Terminal,
};
