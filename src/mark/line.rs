use crate::mark::Mark;
use crate::render::line_renderer::PathInterpolation;
use crate::visual::color::SingleColor;

/// Mark type for line/path charts.
///
/// The `MarkLine` struct defines the visual properties of line elements. 
/// It supports a fluent interface for configuring stroke appearance, 
/// interpolation methods, and statistical smoothing.
#[derive(Clone)]
pub struct MarkLine {
    pub(crate) color: SingleColor,
    pub(crate) stroke_width: f32,
    pub(crate) opacity: f32,
    pub(crate) interpolation: PathInterpolation,
    pub(crate) use_loess: bool,
    pub(crate) loess_bandwidth: f32,
}

impl MarkLine {
    pub(crate) fn new() -> Self {
        Self {
            color: SingleColor::new("black"),
            stroke_width: 2.0,
            opacity: 1.0,
            interpolation: PathInterpolation::Linear,
            use_loess: false,
            loess_bandwidth: 0.75,
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the stroke color of the line. Accepts "red", "#hex", etc.
    pub fn color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the thickness of the line.
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    /// Sets the opacity of the line mark.
    /// 
    /// Value should be between 0.0 (transparent) and 1.0 (opaque).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the interpolation method for connecting data points.
    /// 
    /// Accepts `PathInterpolation` variants or string literals like "basis" or "step".
    pub fn interpolation(mut self, interpolation: impl Into<PathInterpolation>) -> Self {
        self.interpolation = interpolation.into();
        self
    }

    /// Enables or disables LOESS (Locally Estimated Scatterplot Smoothing).
    pub fn use_loess(mut self, use_loess: bool) -> Self {
        self.use_loess = use_loess;
        self
    }

    /// Sets the bandwidth parameter for LOESS smoothing.
    /// 
    /// Controls the smoothness; value should be between 0.0 and 1.0.
    pub fn loess_bandwidth(mut self, bandwidth: f32) -> Self {
        self.loess_bandwidth = bandwidth.clamp(0.0, 1.0);
        self
    }
}

impl Default for MarkLine {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkLine {
    fn mark_type(&self) -> &'static str {
        "line"
    }
}