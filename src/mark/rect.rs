use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for rectangle/heatmap charts
///
/// The `MarkRect` struct defines the visual properties of rectangular elements used in
/// heatmap visualizations and other rectangle-based chart types. It controls the
/// appearance of rectangles including their fill color, opacity, stroke, and stroke width.
///
/// Rectangle marks are commonly used in heatmaps where data values are represented by
/// colored rectangles arranged in a grid pattern. They can also be used for other
/// visualizations like treemaps or tile-based layouts where rectangular areas represent
/// data values or categories.
///
/// # Color Handling
///
/// In rectangle/heatmap charts, colors are typically assigned based on data values
/// using a continuous color mapping system. Each rectangle's fill color represents
/// the magnitude of the data value it represents. When color encoding is used,
/// rectangles are automatically assigned colors from a colormap based on their
/// normalized data values. The `color` field in this struct serves as a fallback
/// when no color encoding is provided, though in typical heatmap usage, color
/// encoding is essential for conveying the data values.
#[derive(Clone)]
pub struct MarkRect {
    pub(crate) color: Option<SingleColor>,
    pub(crate) opacity: f64,
    pub(crate) stroke: Option<SingleColor>,
    pub(crate) stroke_width: f64,
}

impl MarkRect {
    pub(crate) fn new() -> Self {
        Self {
            color: Some(SingleColor::new("black")),
            opacity: 1.0,
            stroke: Some(SingleColor::new("white")),
            stroke_width: 0.0,
        }
    }
}

impl Default for MarkRect {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkRect {
    fn mark_type(&self) -> &'static str {
        "rect"
    }
}
