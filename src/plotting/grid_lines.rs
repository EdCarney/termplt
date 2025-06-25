use super::{
    common::{Drawable, FloatConvertable, Graphable, MaskPoints},
    limits::Limits,
    line::{Line, LineStyle},
    line_positioning::LinePositioning,
    point::Point,
};
use crate::common::Result;

#[derive(Debug, Clone)]
pub enum GridLines {
    XOnly(LineStyle),
    YOnly(LineStyle),
    XY(LineStyle),
}

impl GridLines {
    pub fn get_mask<T: FloatConvertable + Graphable>(
        &self,
        limits: Limits<T>,
    ) -> Result<Vec<MaskPoints>> {
        let limits = limits.convert_to_f64();
        let (limit_span_x, limit_span_y) = limits.span();

        // assume 10 sections; [num lines] = [num sections] - 1
        let mut mask_points = Vec::new();
        let num_sections = 10;
        let interval_x = limit_span_x / num_sections.convert_to_f64();
        let interval_y = limit_span_y / num_sections.convert_to_f64();

        for i in 1..=num_sections {
            match self {
                GridLines::XOnly(line_style) => {
                    // start at 1 to skip the first line
                    let pos = LinePositioning::Horizontal {
                        start: Point::new(limits.min().x, interval_y * i as f64),
                        length: limit_span_x,
                    };
                    mask_points.extend(Line::new(pos, *line_style).get_mask()?);
                }
                GridLines::YOnly(line_style) => {
                    // start at 1 to skip the first line
                    let pos = LinePositioning::Vertical {
                        start: Point::new(interval_x * i as f64, limits.min().y),
                        length: limit_span_y,
                    };
                    mask_points.extend(Line::new(pos, *line_style).get_mask()?);
                }
                GridLines::XY(line_style) => {
                    let pos_horz = LinePositioning::Horizontal {
                        start: Point::new(limits.min().x, interval_y * i as f64),
                        length: limit_span_x,
                    };

                    let pos_vert = LinePositioning::Vertical {
                        start: Point::new(interval_x * i as f64, limits.min().y),
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
