use super::colors;
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
            size: 2,
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
