use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for arc-shaped elements (Pie/Donut charts).
///
/// The `MarkArc` struct defines the visual properties of arcs.
/// It supports a fluent interface for configuring fill color, opacity, 
/// stroke properties, and the inner radius ratio for donut charts.
#[derive(Clone)]
pub struct MarkArc {
    pub(crate) color: SingleColor,
    pub(crate) opacity: f32,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f32,
    pub(crate) inner_radius_ratio: f32,
}

impl MarkArc {
    pub(crate) fn new() -> Self {
        Self {
            color: SingleColor::new("black"),
            opacity: 1.0,
            stroke: SingleColor::new("white"),
            stroke_width: 1.0,
            inner_radius_ratio: 0.0,
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the fill color of the arc. Accepts "red", "#hex", etc.
    pub fn color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the opacity of the arc mark.
    /// 
    /// Value should be between 0.0 (transparent) and 1.0 (opaque).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the stroke color. Use "none" to disable.
    pub fn stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the thickness of the arc's outline.
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    /// Sets the inner radius ratio (0.0 for Pie, > 0.0 for Donut).
    /// 
    /// Value is clamped between 0.0 and 1.0.
    pub fn inner_radius_ratio(mut self, ratio: f32) -> Self {
        self.inner_radius_ratio = ratio.clamp(0.0, 1.0);
        self
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