use std::ops::{Add, Div, Mul, Sub};

pub trait Graphable<T>:
    PartialOrd
    + PartialEq
    + Add<Output = T>
    + Sub<Output = T>
    + Mul<Output = T>
    + Div<Output = T>
    + Clone
    + Copy
    + Sized
{
}

impl<T> Graphable<T> for T where
    T: PartialOrd
        + PartialEq
        + Add<Output = T>
        + Sub<Output = T>
        + Mul<Output = T>
        + Div<Output = T>
        + Clone
        + Copy
        + Sized
{
}
