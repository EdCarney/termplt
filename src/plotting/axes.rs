use super::{
    common::{Drawable, FloatConvertable, Graphable, MaskPoints},
    grid_lines::NUM_GRID_SECTIONS,
    limits::Limits,
    line::{Line, LineStyle},
    line_positioning::LinePositioning,
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

    pub fn get_labels<T: FloatConvertable + Graphable>(
        &self,
        canvas_limits: &Limits<T>,
        graph_limits: &Limits<T>,
    ) -> Result<Vec<Label>> {
        let canvas_limits = canvas_limits.convert_to_f64();
        let graph_limits = graph_limits.convert_to_f64();

        let (x_starts_graph, y_starts_graph) = graph_limits.chunk(NUM_GRID_SECTIONS);
        let (x_starts_canvas, y_starts_canvas) = canvas_limits.chunk(NUM_GRID_SECTIONS);

        let x_starts = x_starts_graph.iter().zip(x_starts_canvas);
        let y_starts = y_starts_graph.iter().zip(y_starts_canvas);

        let x_labels = |line_style: &LineStyle| {
            x_starts
                .map(|(graph_start, canvas_start)| {
                    let txt = Text::from_number(graph_start.x, 3, self.style.clone());
                    let x = canvas_start.x;
                    let y = canvas_start.y
                        - line_style.thickness().convert_to_f64() * 2.
                        - (txt.height() as f64 / 2.);
                    Label::new(txt, TextPositioning::Centered(Point::new(x, y).floor()))
                })
                .collect::<Vec<_>>()
        };
        let y_labels = |line_style: &LineStyle| {
            y_starts
                .map(|(graph_start, canvas_start)| {
                    let txt = Text::from_number(graph_start.y, 3, self.style.clone());
                    let x = canvas_start.x
                        - line_style.thickness().convert_to_f64() * 2.
                        - (txt.width() as f64 / 2.);
                    let y = canvas_start.y;
                    Label::new(txt, TextPositioning::Centered(Point::new(x, y).floor()))
                })
                .collect::<Vec<_>>()
        };

        // canvas limits define where the points will be drawn; the graph limits are only used to
        // know the values of the labels
        let labels = match &self.positioning {
            AxesPositioning::XOnly(line_style) => x_labels(line_style),
            AxesPositioning::YOnly(line_style) => y_labels(line_style),
            AxesPositioning::XY(line_style) => x_labels(line_style)
                .into_iter()
                .chain(y_labels(line_style))
                .collect::<Vec<_>>(),
        };
        Ok(labels)
    }

    pub fn get_mask<T: FloatConvertable + Graphable>(
        &self,
        limits: &Limits<T>,
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
