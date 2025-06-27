use super::{
    common::{Drawable, FloatConvertable, Graphable, MaskPoints},
    limits::Limits,
    line::{Line, LineStyle},
    line_positioning::LinePositioning,
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
        let (x_starts, y_starts) = limits.chunk(NUM_GRID_SECTIONS);

        let horz_lines = |length: f64, line_style: &LineStyle| {
            y_starts
                .into_iter()
                .map(|start| LinePositioning::Horizontal { start, length })
                .flat_map(|pos| Line::new(pos, *line_style).get_mask().unwrap())
                .collect::<Vec<_>>()
        };
        let vert_lines = |length: f64, line_style: &LineStyle| {
            x_starts
                .into_iter()
                .map(|start| LinePositioning::Vertical { start, length })
                .flat_map(|pos| Line::new(pos, *line_style).get_mask().unwrap())
                .collect::<Vec<_>>()
        };

        let mask_points = match self {
            GridLines::XOnly(line_style) => horz_lines(limit_span_x, line_style),
            GridLines::YOnly(line_style) => vert_lines(limit_span_y, line_style),
            GridLines::XY(line_style) => horz_lines(limit_span_x, line_style)
                .into_iter()
                .chain(vert_lines(limit_span_y, line_style))
                .collect::<Vec<_>>(),
        };
        Ok(mask_points)
    }
}
