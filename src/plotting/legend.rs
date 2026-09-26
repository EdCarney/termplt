//! The legend: which series it lists, how big it is, where it goes, and its frame.

/// Where the legend goes inside the plot area. The fixed locations are matplotlib's `loc`
/// values, in the same order; [`LegendLocation::Best`] picks the one covering the least data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LegendLocation {
    /// The fixed location covering the least data, as matplotlib's `loc="best"` picks it.
    #[default]
    Best,
    /// The upper right corner.
    UpperRight,
    /// The upper left corner.
    UpperLeft,
    /// The lower left corner.
    LowerLeft,
    /// The lower right corner.
    LowerRight,
    /// Centered on the right edge: the same place as [`LegendLocation::CenterRight`].
    Right,
    /// Centered on the left edge.
    CenterLeft,
    /// Centered on the right edge.
    CenterRight,
    /// Centered on the bottom edge.
    LowerCenter,
    /// Centered on the top edge.
    UpperCenter,
    /// The middle of the plot.
    Center,
}
