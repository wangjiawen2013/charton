use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for histogram charts
///
/// The `MarkHist` struct defines the visual properties of bar elements used in
/// histogram charts. It controls the appearance of histogram bars including their
/// color, opacity, stroke, and stroke width.
///
/// Histogram marks are used to display the distribution of continuous data by
/// grouping values into bins and showing the frequency or density of observations
/// in each bin. Unlike regular bar charts, histogram bars are typically adjacent
/// to each other to emphasize the continuous nature of the data.
///
/// # Color Handling
///
/// In histogram charts, colors can be assigned based on data categories or groups.
/// When color encoding is used, each histogram bar or group of bars will be assigned
/// a color from the palette system to distinguish between different data series or
/// categories. When no explicit color encoding is provided, the `color` field in
/// this struct serves as the default fill color for all bars. For grouped histograms,
/// different series are automatically assigned different colors from the palette
/// to distinguish them.
#[derive(Debug, Clone)]
pub struct MarkHist {
    pub(crate) color: Option<SingleColor>,
    pub(crate) opacity: f64,
    pub(crate) stroke: Option<SingleColor>,
    pub(crate) stroke_width: f64,
}

impl MarkHist {
    /// Create a new bar mark
    pub(crate) fn new() -> Self {
        Self {
            color: Some(SingleColor::new("black")),
            opacity: 1.0,
            stroke: Some(SingleColor::new("black")),
            stroke_width: 0.0,
        }
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
