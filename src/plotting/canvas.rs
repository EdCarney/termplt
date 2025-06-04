use super::{graph::Graph, limits::Limits, point::*, series::Series};
use crate::common::Result;
use rgb::RGB8;
use std::ops::{Add, Div, Mul, Sub};

const CANVAS_ORIGIN: Point<u32> = Point { x: 0, y: 0 };

struct CanvasBuffer {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

trait Canvas<T> {
    type CanvasType;
    fn draw_data(&mut self, graph: &Graph<T>) -> Result<()>;
    fn scale_data(
        &self,
        series: &Series<T>,
        data_limits: &Limits<T>,
    ) -> Result<Series<Self::CanvasType>>;
}

pub struct TerminalCanvas {
    limits: Limits<u64>,
    background: RGB8,
    canvas: Vec<Vec<RGB8>>,
    buffer: CanvasBuffer,
}

impl<T> Canvas<T> for TerminalCanvas
where
    T: From<u64> + Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Div<Output = T> + Copy,
{
    type CanvasType = u64;

    fn draw_data(&mut self, graph: &Graph<T>) -> Result<()> {
        Ok(())
    }

    fn scale_data(
        &self,
        series: &Series<T>,
        data_limits: &Limits<T>,
    ) -> Result<Series<Self::CanvasType>> {
        // subtract the data limit min from the series to make the data min the canvas origin
        // scale the data

        // 200 in data limits => 100 in canvas limits == scale factor of 0.5

        let data_span = data_limits.span();
        let canvas_span = self.limits.span();
        let x_scale_factor = T::from(canvas_span.0) / data_span.0;
        let y_scale_factor = T::from(canvas_span.1) / data_span.1;
        let series = series
            .data()
            .clone()
            .iter()
            .map(|p| *p - data_limits.min)
            .collect();
        todo!()
    }
}
