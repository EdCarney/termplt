use super::{
    common::{Drawable, Graphable},
    graph::Graph,
    limits::Limits,
    point::Point,
    series::Series,
    styles::MarkerStyle,
};
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

#[derive(Debug)]
pub struct TerminalCanvas<T: Graphable> {
    width: u32,
    height: u32,
    background: RGB8,
    canvas: Vec<Vec<RGB8>>,
    buffer: CanvasBuffer,
    graphs: Vec<Graph<T>>,
    limits: Limits<u32>,
}

impl<T> TerminalCanvas<T>
where
    T: Graphable + Into<f64>,
{
    pub fn new(width: u32, height: u32, background: RGB8) -> TerminalCanvas<T> {
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
        let limits = Limits::new(Point::new(0, 0), Point::new(width - 1, height - 1));
        let graphs = vec![];

        TerminalCanvas {
            width,
            height,
            background,
            canvas,
            buffer,
            graphs,
            limits,
        }
    }

    pub fn with_buffer(mut self, buffer_type: BufferType) -> Self {
        self.buffer = CanvasBuffer::new(buffer_type);
        self
    }

    pub fn with_graph(mut self, graph: Graph<T>) -> Self {
        if graph.data().is_empty() {
            panic!("Cannot add empty graph");
        }
        self.graphs.push(graph);
        self
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

    pub fn draw(mut self) -> Result<Self> {
        for graph in &self.graphs {
            match graph.limits() {
                None => (),
                Some(limits) => {
                    for series in graph.data() {
                        let scaled_series = self.scale_data(&series, limits)?;
                        for point in scaled_series.data() {
                            match series.marker_style() {
                                MarkerStyle::FilledSquare {
                                    line_style,
                                    color,
                                    size,
                                } => {
                                    for x in point.x - size..=point.x + size {
                                        for y in point.y - size..=point.y + size {
                                            if self.limits.contains(Point { x, y }) {
                                                // reverse y since higher values means closer to
                                                // the top of the canvas
                                                let y = self.limits.max().y - y;
                                                self.canvas[y as usize][x as usize] = color.clone();
                                            } else {
                                                println!("Skipping point {:?}", Point { x, y });
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
        }
        Ok(self)
    }

    fn set_pixel(&mut self, point: Point<u32>, color: &RGB8) {
        if self.limits.contains(point) {
            let x = point.x;
            let y = self.limits.max().y - point.y;
            self.canvas[y as usize][x as usize] = color.clone();
        } else {
            println!("Skipping point {:?}", point);
        }
    }

    pub fn scale_data(&self, series: &Series<T>, data_limits: &Limits<T>) -> Result<Series<u32>> {
        let canvas_limits = self.get_drawable_limits();
        let canvas_span = canvas_limits.span();
        let data_span = data_limits.span();

        let data_span_x: f64 = data_span.0.into();
        let data_span_y: f64 = data_span.1.into();

        // need to add one to the canvas span since the ends of the drawable area limits are valid
        let x_scale_factor = f64::from(canvas_span.0) / data_span_x;
        let y_scale_factor = f64::from(canvas_span.1) / data_span_y;

        let new_data = series
            .data()
            .iter()
            .map(|p| {
                let p = *p - *data_limits.min();

                let x: f64 = p.x.into();
                let y: f64 = p.y.into();

                let x = x * x_scale_factor;
                let y = y * y_scale_factor;

                let x: u32 = unsafe { x.to_int_unchecked() };
                let y: u32 = unsafe { y.to_int_unchecked() };

                Point { x, y } + *canvas_limits.min()
            })
            .collect::<Vec<_>>();
        Ok(Series::new(&new_data))
    }

    pub fn get_drawable_limits(&self) -> Limits<u32> {
        // set initial point from the buffer
        let min = Point::new(self.buffer.left, self.buffer.bottom);
        let max = Point::new(
            self.width - self.buffer.right - 1,
            self.height - self.buffer.top - 1,
        );

        // update min/max points based on the max marker size
        let largest_marker_sz = self
            .graphs
            .iter()
            .flat_map(|g| g.data())
            .map(|s| {
                u32::max(
                    s.marker_style().bounding_width(),
                    s.marker_style().bounding_height(),
                )
            })
            .max()
            .unwrap();
        let min = min + largest_marker_sz;
        let max = max - largest_marker_sz;

        Limits::new(min, max)
    }

    pub fn get_absolute_limits(&self) -> Limits<u32> {
        let min = Point::new(0, 0);
        let max = Point::new(self.width - 1, self.height - 1);
        Limits::new(min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plotting::{colors, series::Series};

    #[test]
    fn empty_canvas() {
        TerminalCanvas::new(100, 100, colors::BLACK)
            .with_graph(Graph::<u32>::new())
            .draw()
            .unwrap();
    }

    #[test]
    fn single_series() {
        let points = (0..=5).map(|x| Point::new(x, x)).collect::<Vec<Point<_>>>();
        TerminalCanvas::new(100, 100, colors::BLACK)
            .with_buffer(BufferType::Uniform(5))
            .with_graph(Graph::new().with_series(Series::new(&points)))
            .draw()
            .unwrap();
    }
}
