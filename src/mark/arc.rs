use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Represents an arc-shaped mark for pie and donut charts
///
/// The `MarkArc` struct defines the visual properties of arc-shaped elements
/// used in pie charts and donut charts. It controls the appearance of individual
/// slices including their color, opacity, stroke, and inner radius ratio which
/// determines whether the chart is a pie chart (full circle) or donut chart (with hole).
///
/// This mark type is specifically designed for polar coordinate visualizations
/// where data is represented as segments of a circle, with the angle of each segment
/// proportional to its value.
///
/// # Color Handling
///
/// In pie/donut charts, each slice typically represents a different category and is
/// automatically assigned a color from the palette system. When no explicit color
/// encoding is provided, the chart will cycle through colors from the default palette
/// to distinguish between slices. The `color` field in this struct serves as a fallback
/// when palette-based coloring is not used.
#[derive(Clone)]
pub struct MarkArc {
    pub(crate) color: Option<SingleColor>,
    pub(crate) opacity: f64,
    pub(crate) stroke: Option<SingleColor>,
    pub(crate) stroke_width: f64,
    pub(crate) inner_radius_ratio: f64, // Ratio of inner radius to outer radius for donut chart
}

impl MarkArc {
    pub(crate) fn new() -> Self {
        Self {
            color: Some(SingleColor::new("black")),
            opacity: 1.0,
            stroke: Some(SingleColor::new("white")),
            stroke_width: 1.0,
            inner_radius_ratio: 0.0, // Default to 0.0 for regular pie chart
        }
    }
}

impl Default for MarkArc {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkArc {
    fn mark_type(&self) -> &'static str {
        "arc"
    }
}
