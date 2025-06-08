use super::{colors, common::Drawable, limits::Limits, point::Point};
use crate::common::Result;
use rgb::RGB8;

#[derive(Debug, PartialEq)]
pub enum LineStyle {
    None,
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
