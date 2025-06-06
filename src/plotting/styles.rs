use super::{colors, common::Drawable, limits::Limits, point::Point};
use rgb::RGB8;

#[derive(Debug, PartialEq)]
pub enum MarkerStyle {
    FilledSquare {
        line_style: Option<LineStyle>,
        color: RGB8,
        size: u32,
    },
    HollowSquare {
        line_style: LineStyle,
        size: u32,
    },
}

impl MarkerStyle {
    pub const fn default() -> MarkerStyle {
        Self::FilledSquare {
            line_style: None,
            color: colors::WHITE,
            size: 1,
        }
    }

    pub const fn default_with_size(size: u32) -> MarkerStyle {
        Self::FilledSquare {
            line_style: None,
            color: colors::WHITE,
            size,
        }
    }
}

impl Drawable for MarkerStyle {
    fn bounding_width(&self) -> u32 {
        match self {
            Self::FilledSquare {
                line_style: _,
                color: _,
                size,
            } => size.clone(),
            Self::HollowSquare {
                line_style: _,
                size,
            } => size.clone(),
        }
    }

    fn bounding_height(&self) -> u32 {
        match self {
            Self::FilledSquare {
                line_style: _,
                color: _,
                size,
            } => size.clone(),
            Self::HollowSquare {
                line_style: _,
                size,
            } => size.clone(),
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
            color: colors::WHITE,
            thickness: 2,
        }
    }

    pub const fn default_with_thickness(thickness: u32) -> LineStyle {
        Self::Solid {
            color: colors::WHITE,
            thickness,
        }
    }
}
