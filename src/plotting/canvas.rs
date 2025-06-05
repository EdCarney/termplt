use super::{common::Graphable, graph::Graph, limits::Limits, point::Point, styles::MarkerStyle};
use crate::common::Result;
use rgb::RGB8;

#[derive(Debug)]
struct CanvasBuffer {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

trait Canvas<T: Graphable<T>> {
    fn draw_data(&mut self, graph: &Graph<T>) -> Result<()>;
    fn scale_data(
        &self,
        series_data: &[Point<T>],
        data_limits: &Limits<T>,
    ) -> Result<Vec<Point<u32>>>;
}

#[derive(Debug)]
pub struct TerminalCanvas {
    limits: Limits<u32>,
    background: RGB8,
    canvas: Vec<Vec<RGB8>>,
    buffer: CanvasBuffer,
}

impl TerminalCanvas {
    pub fn new(width: usize, height: usize, background: RGB8) -> TerminalCanvas {
        let mut canvas = Vec::with_capacity(height);
        for _ in 0..height {
            canvas.push(vec![background; width]);
        }
        let limit_min = Point { x: 0, y: 0 };
        let limit_max = Point {
            x: width as u32,
            y: height as u32,
        };
        let limits = Limits::new(limit_min, limit_max);

        let buffer = CanvasBuffer {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };

        TerminalCanvas {
            limits,
            background,
            canvas,
            buffer,
        }
    }
}

impl<T> Canvas<T> for TerminalCanvas
where
    T: Graphable<T> + From<u32> + Into<u32>,
{
    // TODO: calculate drawing limits considering the size of the markers (so that we don't go out
    // of bounds) and the canvas buffer
    fn draw_data(&mut self, graph: &Graph<T>) -> Result<()> {
        match graph.limits() {
            None => (),
            Some(limits) => {
                for series in graph.data() {
                    let scaled_points = self.scale_data(series.data(), limits)?;
                    for point in scaled_points {
                        match series.marker_style() {
                            MarkerStyle::FilledSquare {
                                line_style,
                                color,
                                size,
                            } => {
                                for x in point.x - size..=point.x + size {
                                    for y in point.y - size..=point.y + size {
                                        if self.limits.contains(Point { x, y }) {
                                            self.canvas[y as usize][x as usize] = color.clone();
                                        }
                                    }
                                }
                            }
                            _ => panic!("Not implemented!"),
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn scale_data(
        &self,
        series_data: &[Point<T>],
        data_limits: &Limits<T>,
    ) -> Result<Vec<Point<u32>>> {
        // subtract the data limit min from the series to make the data min the canvas origin
        // scale the data

        // 200 in data limits => 100 in canvas limits == scale factor of 0.5

        let data_span = data_limits.span();
        let canvas_span = self.limits.span();
        let x_scale_factor = T::from(canvas_span.0) / data_span.0;
        let y_scale_factor = T::from(canvas_span.1) / data_span.1;
        Ok(series_data
            .iter()
            .map(|p| {
                let mut p = *p - *data_limits.min();
                p.x = p.x * x_scale_factor;
                p.y = p.y * y_scale_factor;
                Point {
                    x: u32::try_from(p.x * x_scale_factor).unwrap(),
                    y: u32::try_from(p.y * y_scale_factor).unwrap(),
                }
            })
            .collect())
    }
}
