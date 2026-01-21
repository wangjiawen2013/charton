use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for area charts.
///
/// The `MarkArea` struct defines the visual properties of filled area elements.
/// It supports a fluent interface for configuring fill color, opacity, and 
/// stroke properties of the area boundary.
#[derive(Clone, Debug)]
pub struct MarkArea {
    pub(crate) color: SingleColor,
    pub(crate) opacity: f32,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f32,
}

impl MarkArea {
    pub(crate) fn new() -> Self {
        Self {
            // (0.500, 0.992, 0.553, 0.235), // #fd8d3c
            color: SingleColor::new("gray"),
            opacity: 1.0,
            stroke: SingleColor::new("none"),
            stroke_width: 1.0,
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the fill color of the area. Accepts "red", "#hex", etc.
    pub fn color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the opacity of the area mark.
    /// 
    /// Value should be between 0.0 (transparent) and 1.0 (opaque).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the stroke color for the area boundary. Use "none" to disable.
    pub fn stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the thickness of the area's boundary stroke.
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
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
}