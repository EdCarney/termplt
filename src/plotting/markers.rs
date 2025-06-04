use super::colors;
use rgb::RGB8;

#[derive(Debug, PartialEq)]
pub enum MarkerStyle {
    SingleColorFilledSquare {
        color: RGB8,
        size: u32,
    },
    FilledSquare {
        line_color: RGB8,
        fill_color: RGB8,
        size: u32,
    },
    HollowSquare {
        line_color: RGB8,
        size: u32,
    },
}

impl MarkerStyle {
    pub const fn default() -> MarkerStyle {
        Self::SingleColorFilledSquare {
            color: colors::BLACK,
            size: 2,
        }
    }

    pub const fn default_with_size(size: u32) -> MarkerStyle {
        Self::SingleColorFilledSquare {
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
