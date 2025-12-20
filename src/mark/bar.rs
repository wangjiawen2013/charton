use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for bar charts
///
/// The `MarkBar` struct defines the visual properties of rectangular bar elements
/// used in bar charts, column charts, and histograms. It controls the appearance
/// of individual bars including their color, opacity, stroke, width, and spacing.
///
/// Bar marks are fundamental to many chart types and can be oriented horizontally
/// or vertically. They support grouping and stacking configurations for comparing
/// multiple categories or showing part-to-whole relationships.
///
/// # Color Handling
///
/// In bar charts, colors are typically assigned based on data categories or groups.
/// When color encoding is used, each bar or group of bars will be assigned a color
/// from the palette system to distinguish between different data series or categories.
/// When no explicit color encoding is provided, the `color` field in this struct
/// serves as the default fill color for all bars. For grouped or stacked bar charts,
/// different series are automatically assigned different colors from the palette
/// to distinguish them.
#[derive(Debug, Clone)]
pub struct MarkBar {
    pub(crate) color: Option<SingleColor>,
    pub(crate) opacity: f64,
    pub(crate) stroke: Option<SingleColor>,
    pub(crate) stroke_width: f64,
    pub(crate) width: f64,
    pub(crate) spacing: f64, // Add this field for spacing between bars in a group like boxplot
    pub(crate) span: f64,    // Add this field for total span of a group like boxplot
}

impl MarkBar {
    /// Create a new bar mark
    pub(crate) fn new() -> Self {
        Self {
            color: Some(SingleColor::new("steelblue")),
            opacity: 1.0,
            stroke: Some(SingleColor::new("black")),
            stroke_width: 1.0,
            width: 0.5,   // The maximal width of a bar, the actual width may be smaller
            spacing: 0.0, // Default space(spacing*(actual width)) between bars in a group
            span: 0.7,    // Default total span for a group
        }
    }
}

impl Default for MarkBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkBar {
    fn mark_type(&self) -> &'static str {
        "bar"
    }
}
