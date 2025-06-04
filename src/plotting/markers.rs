use super::colors;
use crate::common::Result;
use std::ops::Range;
use rgb::RGB8;

struct CanvasPoint { x: u32, y: u32 }
struct CanvasRange { x_range: Range<u32>, y_range: Range<u32> }

pub trait Canvas {
    fn draw_point(point: CanvasPoint, color: RGB8) -> Result<()>;
    fn draw_range(range: CanvasRange, color: RGB8) -> Result<()>;
    fn draw_marker(&self, at: CanvasPoint, style: MarkerStyle) -> Result<()>;
    fn draw_line(&self, between: CanvasRange, style: LineStyle) -> Result<()>;
}

#[derive(Debug, PartialEq)]
pub enum MarkerStyle {
    FilledSquare {
        line_style: Option<LineStyle>,
        color: RGB8,
        size: u32,
    },
    HollowSquare {
        line_style: LineStyle
        size: u32,
    },
}

impl MarkerStyle {
    pub const fn default() -> MarkerStyle {
        Self::FilledSquare {
            line_style: None,
            color: colors::BLACK,
            size: 2,
        }
    }

    pub const fn default_with_size(size: u32) -> MarkerStyle {
        Self::FilledSquare {
            line_style: None,
            color: colors::BLACK,
            size,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum LineStyle {
    Solid { color: RGB8, thickness: u32 },
}

impl LineStyle {
    pub const fn default() -> LineStyle {
        Self::Solid {
            color: colors::BLACK,
            thickness: 2,
        }
    }

    pub const fn default_with_thickness(thickness: u32) -> LineStyle {
        Self::Solid {
            color: colors::BLACK,
            thickness,
        }
    }
}
