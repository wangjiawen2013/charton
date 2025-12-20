use crate::mark::Mark;
use crate::render::line_renderer::PathInterpolation;
use crate::visual::color::SingleColor;

/// Mark type for line charts
///
/// The `MarkLine` struct defines the visual properties of line elements used in
/// line charts and time series visualizations. It controls the appearance of lines
/// including their color, stroke width, opacity, and optional LOESS smoothing.
///
/// Line marks are used to display trends and patterns in data over continuous
/// intervals or time periods. They connect individual data points to show the
/// overall shape of the data distribution and can be enhanced with smoothing
/// algorithms for better visualization of underlying patterns.
///
/// # Color Handling
///
/// In line charts, colors can be assigned based on data categories or groups.
/// When color encoding is used, each line will be assigned a color from the
/// palette system to distinguish between different data series or categories.
/// When no explicit color encoding is provided, the `color` field in this struct
/// serves as the default stroke color for all lines. For multi-series line charts,
/// different series are automatically assigned different colors from the palette
/// to distinguish them.
#[derive(Clone)]
pub struct MarkLine {
    /// Color of the line
    pub color: Option<SingleColor>,
    /// Width of the line stroke
    pub stroke_width: f64,
    /// Opacity of the line
    pub opacity: f64,
    /// Whether to apply LOESS smoothing
    pub use_loess: bool,
    /// Bandwidth parameter for LOESS smoothing
    pub loess_bandwidth: f64,
    /// Interpolation method for connecting points
    pub interpolation: PathInterpolation,
}

impl MarkLine {
    pub(crate) fn new() -> Self {
        Self {
            color: Some(SingleColor::new("black")),
            stroke_width: 2.0,
            opacity: 1.0,
            use_loess: false,
            loess_bandwidth: 0.75,
            interpolation: PathInterpolation::Linear,
        }
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
