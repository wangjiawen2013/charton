use crate::mark::Mark;
use crate::visual::color::SingleColor;
use crate::visual::shape::PointShape;

/// Mark type for area charts
///
/// The `MarkArea` struct defines the visual properties of area elements used in
/// area charts and stacked area charts. It controls the appearance of filled regions
/// under data lines, including their color, opacity, and stroke properties.
///
/// Area marks are typically used to visualize cumulative data or to show the
/// contribution of different categories to a total over time or across categories.
/// They provide a filled area between the data line and a baseline (usually zero).
///
/// # Color Handling
///
/// In area charts, colors are typically assigned based on data categories or groups.
/// When color encoding is used, each area will be assigned a color from the palette
/// system to distinguish between different data series. When no explicit color
/// encoding is provided, the `color` field in this struct serves as the default
/// fill color for all areas. For stacked area charts, different series are
/// automatically assigned different colors from the palette to distinguish them.
#[derive(Debug, Clone)]
pub struct MarkArea {
    pub(crate) color: Option<SingleColor>,
    pub(crate) opacity: f64,
    pub(crate) stroke: Option<SingleColor>,
    pub(crate) stroke_width: f64,
}

impl MarkArea {
    /// Create a new area mark
    pub(crate) fn new() -> Self {
        Self {
            color: Some(SingleColor::new("gray")),
            opacity: 1.0,
            stroke: None,
            stroke_width: 1.0,
        }
    }
}

impl Default for MarkArea {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkArea {
    fn mark_type(&self) -> &'static str {
        "area"
    }

    fn stroke(&self) -> Option<&SingleColor> {
        self.stroke.as_ref()
    }

    fn opacity(&self) -> f64 {
        self.opacity
    }

    fn shape(&self) -> PointShape {
        PointShape::Circle
    }
}
