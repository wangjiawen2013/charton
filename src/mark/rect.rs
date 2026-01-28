use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for rectangle/heatmap charts.
///
/// The `MarkRect` struct defines the visual properties of rectangular elements.
/// It supports a fluent interface for configuring fill color, opacity, 
/// and boundary stroke properties.
#[derive(Clone)]
pub struct MarkRect {
    pub(crate) color: SingleColor,
    pub(crate) opacity: f64,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f64,
}

impl MarkRect {
    pub(crate) fn new() -> Self {
        Self {
            // (0.500, 0.992, 0.553, 0.235), // #fd8d3c
            color: SingleColor::new("black"),
            opacity: 1.0,
            stroke: SingleColor::new("white"),
            stroke_width: 0.0,
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the fill color of the rectangle. Accepts "red", "#hex", etc.
    pub fn with_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the opacity of the rectangle mark.
    /// 
    /// Value should be between 0.0 (transparent) and 1.0 (opaque).
    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the stroke color of the rectangle boundary. Use "none" to disable.
    pub fn with_stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the thickness of the rectangle's outline.
    pub fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
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