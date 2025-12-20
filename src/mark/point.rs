use crate::mark::Mark;
use crate::visual::color::SingleColor;
use crate::visual::shape::PointShape;

/// Mark type for point/scatter charts
///
/// The `MarkPoint` struct defines the visual properties of point elements used in
/// scatter plots and point-based visualizations. It provides default values for
/// color, shape, size, opacity, and stroke properties, though these are typically
/// overridden by encoding channels to create data-driven visualizations.
///
/// Point marks are fundamental to many chart types and are used to display individual
/// data points in Cartesian coordinate systems. They support various shapes, sizes,
/// and colors to encode multiple dimensions of data simultaneously.
///
/// # Color Handling
///
/// In point/scatter charts, colors can be assigned based on data categories or groups.
/// When color encoding is used, each point will be assigned a color from the
/// palette system to distinguish between different data series or categories.
/// When no explicit color encoding is provided, the `color` field in this struct
/// serves as the default fill color for all points. For multi-series scatter plots,
/// different series are automatically assigned different colors from the palette
/// to distinguish them. Points can also have separate stroke colors for additional
/// visual distinction.
#[derive(Clone)]
pub struct MarkPoint {
    pub(crate) color: Option<SingleColor>,
    pub(crate) shape: PointShape,
    pub(crate) size: f64,
    pub(crate) opacity: f64,
    pub(crate) stroke: Option<SingleColor>,
    pub(crate) stroke_width: f64,
}

impl MarkPoint {
    pub(crate) fn new() -> Self {
        Self {
            color: Some(SingleColor::new("black")),
            shape: PointShape::Circle,
            size: 3.0,
            opacity: 1.0,
            stroke: None,
            stroke_width: 0.0,
        }
    }
}

impl Default for MarkPoint {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkPoint {
    fn mark_type(&self) -> &'static str {
        "point"
    }
}
