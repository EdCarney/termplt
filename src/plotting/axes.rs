use super::{
    common::{Drawable, FloatConvertable, Graphable, MaskPoints},
    limits::Limits,
    line::{Line, LineStyle},
    line_positioning::LinePositioning,
    numbers,
    point::Point,
    text::{Label, Text, TextPositioning, TextStyle},
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
    style: TextStyle,
}

impl Axes {
    pub fn new(positioning: AxesPositioning) -> Axes {
        let style = TextStyle::default();
        Axes { positioning, style }
    }

    pub fn positioning(&self) -> &AxesPositioning {
        &self.positioning
    }

    pub fn style(&self) -> &TextStyle {
        &self.style
    }

    pub fn get_mask<T: FloatConvertable + Graphable>(
        &self,
        limits: Limits<T>,
    ) -> Result<Vec<MaskPoints>> {
        let limits = limits.convert_to_f64();
        let (limit_span_x, limit_span_y) = limits.span();

        // thickness is applied in both directions from the line center, so shift the start of
        // the line in the appropriate direction to ensure it will not go into the data area
        match &self.positioning {
            AxesPositioning::XOnly(line_style) => {
                let start = Point::new(
                    limits.min().x,
                    limits.min().y - line_style.thickness().convert_to_f64(),
                );
                let length = limit_span_x;
                let pos = LinePositioning::Horizontal { start, length };
                let line = Line::new(pos, *line_style);

                // x points will increase in the x direction and have consistent y position
                let label_txt = Text::from_number(start.x, 3, self.style.clone());
                let label_x = limits.min().x;
                let label_y = limits.min().y
                    - line_style.thickness().convert_to_f64() * 2.
                    - (label_txt.height() as f64 / 2.);
                let label_center = Point::new(label_x, label_y).floor();
                let label = Label::new(label_txt, TextPositioning::Centered(label_center));

                let mask = vec![line.get_mask()?, label.get_mask()?]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                Ok(mask)
            }
            AxesPositioning::YOnly(line_style) => {
                let start = Point::new(
                    limits.min().x - line_style.thickness().convert_to_f64(),
                    limits.min().y,
                );
                let length = limit_span_y;
                let pos = LinePositioning::Vertical { start, length };
                let line = Line::new(pos, *line_style);

                // labels will increase in the y direction and have consistent x position
                let label_txt = Text::from_number(start.y, 3, self.style.clone());
                let label_x = limits.min().x
                    - line_style.thickness().convert_to_f64() * 2.
                    - (label_txt.width() as f64 / 2.);
                let label_y = limits.min().y;
                let label_center = Point::new(label_x, label_y).floor();
                let label = Label::new(label_txt, TextPositioning::Centered(label_center));

                let mask = vec![line.get_mask()?, label.get_mask()?]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                Ok(mask)
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
