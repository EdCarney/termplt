use super::{
    axes::AxesPositioning,
    common::{Drawable, FloatConvertable, Graphable},
    graph::Graph,
    limits::Limits,
    point::Point,
    text::{Label, Text},
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
    labels: Option<Vec<Label>>,
    limits: Limits<u32>,
}

impl<T> TerminalCanvas<T>
where
    T: Graphable + FloatConvertable,
{
    pub fn new(width: u32, height: u32, background: RGB8) -> TerminalCanvas<T> {
        TerminalCanvas {
            canvas: Canvas::new(width, height, background),
            buffer: CanvasBuffer::new(BufferType::None),
            graph: None,
            labels: None,
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

    pub fn with_label(mut self, label: Label) -> Self {
        self.labels = if let Some(mut labels) = self.labels {
            labels.push(label);
            Some(labels)
        } else {
            Some(vec![label])
        };
        self
    }

    pub fn get_bytes(&self) -> Vec<u8> {
        self.canvas.get_bytes()
    }

    /// Consumes all drawable assets and draws them on the canvas.
    pub fn draw(mut self) -> Result<Self> {
        let canvas_limits = self.get_drawable_limits().convert_to_f64();

        if let Some(graph) = self.graph.take() {
            graph
                .scale(canvas_limits)
                .get_mask()?
                .iter()
                .for_each(|mask| self.canvas.set_pixels(&mask.points, &mask.color));
        }

        if let Some(text) = self.labels.take() {
            text.iter()
                .flat_map(|txt| txt.get_mask().unwrap())
                .for_each(|mask| self.canvas.set_pixels(&mask.points, &mask.color));
        }

        Ok(self)
    }

    pub fn get_drawable_limits(&self) -> Limits<u32> {
        // set initial point from the buffer
        let mut min = Point::new(self.buffer.left, self.buffer.bottom);
        let mut max = self.limits.max().clone() - Point::new(self.buffer.right, self.buffer.top);

        if let Some(graph) = &self.graph {
            let largest_marker_sz = graph
                .data()
                .iter()
                .map(|s| s.marker_style().size())
                .max()
                .unwrap();

            // axes thickness in x/y pixels
            let axes_thickness = match graph.axes() {
                Some(axes) => match axes.positioning() {
                    AxesPositioning::XOnly(line_style) => (0, 2 * line_style.thickness()),
                    AxesPositioning::YOnly(line_style) => (2 * line_style.thickness(), 0),
                    AxesPositioning::XY(line_style) => {
                        (2 * line_style.thickness(), 2 * line_style.thickness())
                    }
                },
                None => (0, 0),
            };

            // note that axes and markers can overlap; so use the larger of marker/axes as bounds
            let mut min_x = min.x + u32::max(largest_marker_sz, axes_thickness.0);
            let mut min_y = min.y + u32::max(largest_marker_sz, axes_thickness.1);
            let mut max_x = max.x - u32::max(largest_marker_sz, axes_thickness.0);
            let mut max_y = max.y - u32::max(largest_marker_sz, axes_thickness.1);

            // include axes text

            min = Point::new(min_x, min_y);
            max = Point::new(max_x, max_y);
        }

        Limits::new(min, max)
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
