use std::{rc::Rc, sync::Mutex};

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
    None,
    Uniform(u32),
    TopBottom(u32, u32),
    LeftRight(u32, u32),
    TopBottomLeftRight(u32, u32, u32, u32),
}

impl CanvasBuffer {
    pub fn new(buffer_type: BufferType) -> CanvasBuffer {
        match buffer_type {
            BufferType::None => CanvasBuffer::new(BufferType::Uniform(0)),
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
struct Canvas {
    pixels: Vec<Vec<RGB8>>,
    limits: Limits<u32>,
}

impl Canvas {
    pub fn new(width: u32, height: u32, background: RGB8) -> Canvas {
        Canvas {
            pixels: (0..height)
                .map(|_| vec![background; width as usize])
                .collect(),
            limits: Limits::new(Point::new(0, 0), Point::new(width - 1, height - 1)),
        }
    }

    /// Sets the color for a point in the canvas. The provided point should be zero-indexed with
    /// the lower-left corner as (0, 0) and the upper-right cornder as (width - 1, height - 1).
    pub fn set_pixel(&mut self, point: &Point<u32>, color: &RGB8) {
        if self.limits.contains(point) {
            // reverse y since higher values means closer to
            // the top of the canvas
            let x = point.x;
            let y = self.limits.max().y - point.y;
            self.pixels[y as usize][x as usize] = color.clone();
        } else {
            println!("Skipping point {:?}", point);
        }
    }

    /// Sets the color for multiple points in the canvas. The provided points should be zero-indexed
    /// with the lower-left corner as (0, 0) and the upper-right cornder as (width - 1, height - 1).
    pub fn set_pixels(&mut self, points: &[Point<u32>], color: &RGB8) {
        for point in points {
            self.set_pixel(point, color);
        }
    }

    pub fn get_bytes(&self) -> Vec<u8> {
        self.pixels
            .iter()
            .flat_map(|row| {
                row.iter()
                    .flat_map(|&rgb| [rgb.r, rgb.g, rgb.b])
                    .collect::<Vec<u8>>()
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct TerminalCanvas<T: Graphable> {
    canvas: Canvas,
    buffer: CanvasBuffer,
    graph: Option<Graph<T>>,
    limits: Limits<u32>,
}

impl<T> TerminalCanvas<T>
where
    T: Graphable + Into<f64>,
{
    pub fn new(width: u32, height: u32, background: RGB8) -> TerminalCanvas<T> {
        TerminalCanvas {
            canvas: Canvas::new(width, height, background),
            buffer: CanvasBuffer::new(BufferType::None),
            graph: None,
            limits: Limits::new(Point::new(0, 0), Point::new(width - 1, height - 1)),
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
        self.graph = Some(graph);
        self
    }

    pub fn get_bytes(&self) -> Vec<u8> {
        self.canvas.get_bytes()
    }

    pub fn draw(mut self) -> Result<Self> {
        let canvas_limits = self.get_drawable_limits();
        let mut scaled_series = vec![];
        if let Some(ref graph) = self.graph {
            if let Some(limits) = graph.limits() {
                graph.data().iter().for_each(|series| {
                    scaled_series
                        .push(TerminalCanvas::scale_data(&series, &canvas_limits, &limits).unwrap())
                });
            }
        }

        for series in scaled_series {
            for point in series.data() {
                TerminalCanvas::<T>::get_marker_mask(point, series.marker_style())
                    .iter()
                    .for_each(|x| self.canvas.set_pixels(&x.0, &x.1));
            }
        }

        Ok(self)
    }

    fn get_marker_mask(
        center: &Point<u32>,
        marker_style: &MarkerStyle,
    ) -> Vec<(Vec<Point<u32>>, RGB8)> {
        match marker_style {
            MarkerStyle::FilledSquare {
                line_style,
                color,
                size,
            } => {
                let mask = (center.x - size..=center.x + size)
                    .flat_map(|x| (center.y - size..=center.y + size).map(move |y| Point { x, y }))
                    .collect();
                vec![(mask, color.clone())]
            }
            _ => panic!("Not implemented!"),
        }
    }

    fn scale_data(
        series: &Series<T>,
        canvas_limits: &Limits<u32>,
        data_limits: &Limits<T>,
    ) -> Result<Series<u32>> {
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
        let mut min = Point::new(self.buffer.left, self.buffer.bottom);
        let mut max = self.limits.max().clone() - Point::new(self.buffer.right, self.buffer.top);

        // update min/max points based on the max marker size
        if let Some(ref graph) = self.graph {
            let largest_marker_sz = graph
                .data()
                .iter()
                .map(|s| {
                    u32::max(
                        s.marker_style().bounding_width(),
                        s.marker_style().bounding_height(),
                    )
                })
                .max()
                .unwrap();
            min = min + largest_marker_sz;
            max = max - largest_marker_sz;
        }

        Limits::new(min, max)
    }

    pub fn get_absolute_limits(&self) -> Limits<u32> {
        self.limits.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plotting::{colors, series::Series};

    #[test]
    #[should_panic]
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
