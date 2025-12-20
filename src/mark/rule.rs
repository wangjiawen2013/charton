use crate::visual::color::SingleColor;

/// MarkRule represents a rule mark for drawing horizontal or vertical lines
///
/// The `MarkRule` struct defines the visual properties of line elements used to draw
/// horizontal or vertical rules in charts. It controls the appearance of these lines
/// including their color, opacity, and stroke width.
///
/// Rule marks are used for various purposes such as drawing reference lines, thresholds,
/// ranges, or connecting elements in the visualization. They can be positioned using
/// standard encoding channels and support both fixed and data-driven positioning.
///
/// # Color Handling
///
/// In rule charts, colors can be assigned based on data categories or groups.
/// When color encoding is used, each rule line will be assigned a color from the
/// palette system to distinguish between different data series or categories.
/// When no explicit color encoding is provided, the `color` field in this struct
/// serves as the default stroke color for all rule lines. For multi-series rule charts,
/// different series are automatically assigned different colors from the palette
/// to distinguish them.
#[derive(Debug, Clone)]
pub struct MarkRule {
    /// Color of the rule line
    pub(crate) color: Option<SingleColor>,
    /// Opacity of the rule line
    pub(crate) opacity: f64,
    /// Stroke width of the rule line
    pub(crate) stroke_width: f64,
}

impl MarkRule {
    /// Create a new MarkRule with default values
    pub(crate) fn new() -> Self {
        Self {
            color: Some(SingleColor::new("black")),
            opacity: 1.0,
            stroke_width: 1.0,
        }
    }
}

impl Default for MarkRule {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::mark::Mark for MarkRule {
    fn mark_type(&self) -> &'static str {
        "rule"
    }
}
