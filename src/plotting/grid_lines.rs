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
    /// Draws grid lines evenly dividing `limits` into [`NUM_GRID_SECTIONS`] sections.
    pub fn get_mask<T: FloatConvertable + Graphable>(
        &self,
        limits: &Limits<T>,
    ) -> Result<Vec<MaskPoints>> {
        let limits = limits.convert_to_f64();
        let (x_starts, y_starts) = limits.chunk(NUM_GRID_SECTIONS);
        let xs: Vec<f64> = x_starts.iter().map(|p| p.x).collect();
        let ys: Vec<f64> = y_starts.iter().map(|p| p.y).collect();
        self.get_mask_at(&limits, &xs, &ys)
    }

    /// Draws grid lines spanning `limits`: vertical lines at the `xs` positions and horizontal
    /// lines at the `ys` positions (as selected by the variant). Positions are in the same
    /// coordinates as `limits`.
    pub fn get_mask_at(
        &self,
        limits: &Limits<f64>,
        xs: &[f64],
        ys: &[f64],
    ) -> Result<Vec<MaskPoints>> {
        let (span_x, span_y) = limits.span();

        let horz_lines = |line_style: &LineStyle| -> Result<Vec<MaskPoints>> {
            let mut masks = Vec::new();
            for &y in ys {
                let start = Point::new(limits.min().x, y);
                let pos = LinePositioning::Horizontal {
                    start,
                    length: span_x,
                };
                masks.extend(Line::new(pos, *line_style).get_mask()?);
            }
            Ok(masks)
        };
        let vert_lines = |line_style: &LineStyle| -> Result<Vec<MaskPoints>> {
            let mut masks = Vec::new();
            for &x in xs {
                let start = Point::new(x, limits.min().y);
                let pos = LinePositioning::Vertical {
                    start,
                    length: span_y,
                };
                masks.extend(Line::new(pos, *line_style).get_mask()?);
            }
            Ok(masks)
        };

        match self {
            GridLines::XOnly(line_style) => horz_lines(line_style),
            GridLines::YOnly(line_style) => vert_lines(line_style),
            GridLines::XY(line_style) => {
                let mut mask = horz_lines(line_style)?;
                mask.extend(vert_lines(line_style)?);
                Ok(mask)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plotting::colors;

    #[test]
    fn grid_lines_are_drawn_at_given_positions() {
        let limits = Limits::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        let style = LineStyle::Solid {
            color: colors::GRAY,
            thickness: 0,
        };
        let mask = GridLines::XY(style)
            .get_mask_at(&limits, &[10.0, 60.0], &[25.0])
            .unwrap();
        let points: Vec<_> = mask.iter().flat_map(|m| m.points.iter()).collect();
        // one horizontal line of 101 px and two vertical lines of 51 px
        assert_eq!(points.len(), 101 + 2 * 51);
        assert!(points.iter().filter(|p| p.x == 10).count() >= 51);
        assert!(points.iter().filter(|p| p.y == 25).count() >= 101);
    }
}
