use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for histogram charts.
///
/// The `MarkHist` struct defines the visual properties of bars representing binned data.
/// It supports a fluent interface for configuring fill color, opacity, and stroke properties.
/// Histograms typically use these marks to represent frequency distributions.
#[derive(Debug, Clone)]
pub struct MarkHist {
    pub(crate) color: SingleColor,
    pub(crate) opacity: f32,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f32,
}

impl MarkHist {
    pub(crate) fn new() -> Self {
        Self {
            // (0.651, 0.212, 0.012, 1.000), // #a63603
            color: SingleColor::new("black"),
            opacity: 1.0,
            stroke: SingleColor::new("black"),
            stroke_width: 0.0,
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the fill color of the histogram bars. Accepts "red", "#hex", etc.
    pub fn color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the opacity of the histogram mark.
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
}

impl Default for MarkHist {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkHist {
    fn mark_type(&self) -> &'static str {
        "hist"
    }
}