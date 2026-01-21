use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for bar charts.
///
/// The `MarkBar` struct defines the visual properties of rectangular bar elements.
/// It supports a fluent interface for configuring fill color, stroke, 
/// and dimensional constraints like width, spacing, and span.
#[derive(Debug, Clone)]
pub struct MarkBar {
    pub(crate) color: SingleColor,
    pub(crate) opacity: f32,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f32,
    pub(crate) width: f32,
    pub(crate) spacing: f32,
    pub(crate) span: f32,
}

impl MarkBar {
    pub(crate) fn new() -> Self {
        Self {
            // (0.375, 0.992, 0.682, 0.420), // #fdae6b
            color: SingleColor::new("steelblue"),
            opacity: 1.0,
            stroke: SingleColor::new("black"),
            stroke_width: 1.0,
            width: 0.5,
            spacing: 0.0,
            span: 0.7,
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the fill color of the bars. Accepts "red", "#hex", etc.
    pub fn color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the opacity of the bar mark.
    /// 
    /// Value should be between 0.0 (transparent) and 1.0 (opaque).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the stroke color of the bars. Use "none" to disable.
    pub fn stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the thickness of the bar's outline.
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    /// Sets the maximal width of a bar (as a ratio or absolute value depending on coordinate system).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets the relative spacing between bars within a group.
    /// 
    /// Value is clamped between 0.0 and 1.0.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.clamp(0.0, 1.0);
        self
    }

    /// Sets the total span of a bar group.
    /// 
    /// Value is clamped between 0.0 and 1.0.
    pub fn span(mut self, span: f32) -> Self {
        self.span = span.clamp(0.0, 1.0);
        self
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