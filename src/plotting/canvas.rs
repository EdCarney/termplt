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

pub enum BufferType {
    Uniform(u32),
    TopBottom(u32, u32),
    LeftRight(u32, u32),
    TopBottomLeftRight(u32, u32, u32, u32),
}

impl CanvasBuffer {
    pub fn new(buffer_type: BufferType) -> CanvasBuffer {
        match buffer_type {
            BufferType::Uniform(x) => CanvasBuffer {
                left: x,
                top: x,
                right: x,
                bottom: x,
            },
            _ => panic!("Not implemented"),
        }
    }
}

pub trait Canvas<T: Graphable<T>> {
    fn draw_data(&mut self, graph: &Graph<T>) -> Result<()>;
    fn scale_data(
        &self,
        series_data: &[Point<T>],
        data_limits: &Limits<T>,
    ) -> Result<Vec<Point<u32>>>;
}

#[derive(Debug)]
pub struct TerminalCanvas {
    width: u32,
    height: u32,
    background: RGB8,
    canvas: Vec<Vec<RGB8>>,
    buffer: CanvasBuffer,
}

impl TerminalCanvas {
    pub fn new(width: u32, height: u32, background: RGB8) -> TerminalCanvas {
        let mut canvas = Vec::with_capacity(height as usize);
        for _ in 0..height {
            canvas.push(vec![background; width as usize]);
        }

        let buffer = CanvasBuffer {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };

        TerminalCanvas {
            width,
            height,
            background,
            canvas,
            buffer,
        }
    }

    pub fn with_buffer(mut self, buffer_type: BufferType) -> Self {
        self.buffer = CanvasBuffer::new(buffer_type);
        self
    }

    pub fn get_drawable_limits(&self) -> Limits<u32> {
        let min = Point::new(self.buffer.left, self.buffer.bottom);
        let max = Point::new(
            self.width - self.buffer.right - 1,
            self.height - self.buffer.top - 1,
        );
        Limits::new(min, max)
    }

    pub fn get_bytes(&self) -> Vec<u8> {
        self.canvas
            .iter()
            .flat_map(|row| {
                row.iter()
                    .flat_map(|&rgb| [rgb.r, rgb.g, rgb.b])
                    .collect::<Vec<u8>>()
            })
            .collect()
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
                                let limits = self.get_drawable_limits();
                                for x in point.x - size..=point.x + size {
                                    for y in point.y - size..=point.y + size {
                                        if limits.contains(Point { x, y }) {
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

        let canvas_limits = self.get_drawable_limits();
        let canvas_span = canvas_limits.span();
        let data_span = data_limits.span();
        let x_scale_factor = T::from(canvas_span.0) / data_span.0;
        let y_scale_factor = T::from(canvas_span.1) / data_span.1;
        Ok(series_data
            .iter()
            .map(|p| {
                let p = *p - *data_limits.min();
                let x = canvas_limits.min().x + u32::try_from(p.x * x_scale_factor).unwrap();
                let y = canvas_limits.min().y + u32::try_from(p.y * y_scale_factor).unwrap();
                Point { x, y }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plotting::{colors, series::Series};

    #[test]
    fn empty_canvas() {
        let mut canvas = TerminalCanvas::new(100, 100, colors::BLACK);
        canvas.draw_data(&Graph::<u32>::new()).unwrap();
    }

    #[test]
    fn single_series() {
        let mut canvas =
            TerminalCanvas::new(100, 100, colors::BLACK).with_buffer(BufferType::Uniform(5));
        let mut graph = Graph::<u32>::new();
        let points = (0..=5).map(|x| Point::new(x, x)).collect::<Vec<Point<_>>>();

        graph.add_series(Series::new(&points));
        canvas.draw_data(&graph).unwrap();
    }
}
