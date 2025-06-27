use super::{
    common::{Drawable, FloatConvertable, Graphable, MaskPoints},
    limits::Limits,
    line::{Line, LineStyle},
    line_positioning::LinePositioning,
    point::Point,
};
use crate::common::Result;

pub const NUM_GRID_SECTIONS: u32 = 10;

#[derive(Debug, Clone)]
pub enum GridLines {
    XOnly(LineStyle),
    YOnly(LineStyle),
    XY(LineStyle),
}

impl GridLines {
    pub fn get_mask<T: FloatConvertable + Graphable>(
        &self,
        limits: &Limits<T>,
    ) -> Result<Vec<MaskPoints>> {
        let limits = limits.convert_to_f64();
        let (limit_span_x, limit_span_y) = limits.span();
        let limit_min = limits.min();

        let mut mask_points = Vec::new();
        let interval_x = limit_span_x / NUM_GRID_SECTIONS.convert_to_f64();
        let interval_y = limit_span_y / NUM_GRID_SECTIONS.convert_to_f64();

        for i in 0..=NUM_GRID_SECTIONS {
            match self {
                GridLines::XOnly(line_style) => {
                    let pos = LinePositioning::Horizontal {
                        start: *limit_min + Point::new(0., interval_y * i as f64),
                        length: limit_span_x,
                    };
                    mask_points.extend(Line::new(pos, *line_style).get_mask()?);
                }
                GridLines::YOnly(line_style) => {
                    let pos = LinePositioning::Vertical {
                        start: *limit_min + Point::new(interval_x * i as f64, 0.),
                        length: limit_span_y,
                    };
                    mask_points.extend(Line::new(pos, *line_style).get_mask()?);
                }
                GridLines::XY(line_style) => {
                    let pos_horz = LinePositioning::Horizontal {
                        start: *limit_min + Point::new(0., interval_y * i as f64),
                        length: limit_span_x,
                    };

                    let pos_vert = LinePositioning::Vertical {
                        start: *limit_min + Point::new(interval_x * i as f64, 0.),
                        length: limit_span_y,
                    };

                    mask_points.extend(Line::new(pos_horz, *line_style).get_mask()?);
                    mask_points.extend(Line::new(pos_vert, *line_style).get_mask()?);
                }
            }
        }
        Ok(mask_points)
    }
}
