use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for horizontal or vertical reference lines.
///
/// The `MarkRule` struct defines the visual properties of rule elements. 
/// It supports a fluent interface for configuring the line color, opacity, 
/// and thickness for drawing thresholds, grid lines, or connecting ranges.
#[derive(Debug, Clone)]
pub struct MarkRule {
    pub(crate) color: SingleColor,
    pub(crate) opacity: f32,
    pub(crate) stroke_width: f32,
}

impl MarkRule {
    /// Create a new MarkRule with default values.
    pub(crate) fn new() -> Self {
        Self {
            // (0.875, 0.651, 0.212, 0.012), // #a63603
            color: SingleColor::new("black"),
            opacity: 1.0,
            stroke_width: 1.0,
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the color of the rule line. Accepts "red", "#hex", etc.
    pub fn with_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the opacity of the rule line.
    /// 
    /// Value should be between 0.0 (transparent) and 1.0 (opaque).
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the thickness of the rule line.
    pub fn with_stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }
}

impl Default for MarkRule {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkRule {
    fn mark_type(&self) -> &'static str {
        "rule"
    }
}