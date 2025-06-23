use super::{
    common::{Drawable, FloatConvertable, Graphable, MaskPoints},
    limits::Limits,
    line::{Line, LineStyle},
    line_positioning::LinePositioning,
    point::Point,
};
use crate::common::Result;

#[derive(Debug, Clone)]
pub enum AxesPositioning {
    XOnly(LineStyle),
    YOnly(LineStyle),
    XY(LineStyle),
}

#[derive(Debug, Clone)]
pub struct Axes {
    positioning: AxesPositioning,
}

impl Axes {
    pub fn positioning(&self) -> &AxesPositioning {
        &self.positioning
    }
}

impl AxesPositioning {
    pub fn get_mask<T: FloatConvertable + Graphable>(
        &self,
        limits: Limits<T>,
    ) -> Result<Vec<MaskPoints>> {
        let limits = limits.convert_to_f64();
        let (limit_span_x, limit_span_y) = limits.span();

        // thickness is applied in both directions from the line center, so shift the start of
        // the line in the appropriate direction to ensure it will not go into the data area
        match self {
            AxesPositioning::XOnly(line_style) => {
                let start = Point::new(
                    limits.min().x,
                    limits.min().y - line_style.thickness().convert_to_f64(),
                );
                let length = limit_span_x;
                let pos = LinePositioning::Horizontal { start, length };
                Line::new(pos, *line_style).get_mask()
            }
            AxesPositioning::YOnly(line_style) => {
                let start = Point::new(
                    limits.min().x - line_style.thickness().convert_to_f64(),
                    limits.min().y,
                );
                let length = limit_span_y;
                let pos = LinePositioning::Vertical { start, length };
                Line::new(pos, *line_style).get_mask()
            }
            AxesPositioning::XY(line_style) => {
                let pos_x = LinePositioning::Horizontal {
                    start: Point::new(
                        limits.min().x,
                        limits.min().y - line_style.thickness().convert_to_f64(),
                    ),
                    length: limit_span_x,
                };
                let pos_y = LinePositioning::Vertical {
                    start: Point::new(
                        limits.min().x - line_style.thickness().convert_to_f64(),
                        limits.min().y,
                    ),
                    length: limit_span_y,
                };

                let mut mask = Line::new(pos_x, *line_style).get_mask()?;
                mask.extend(Line::new(pos_y, *line_style).get_mask()?);
                Ok(mask)
            }
        }
    }
}
