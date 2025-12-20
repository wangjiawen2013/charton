use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for error bar charts
///
/// The `MarkErrorBar` struct defines the visual properties of error bar elements
/// used in statistical data visualization. It controls the appearance of error bars
/// including their color, opacity, stroke width, cap length, and center point visibility.
///
/// Error bar marks are used to display the variability or uncertainty of data points,
/// typically showing standard deviation, standard error, or confidence intervals.
/// They are commonly overlaid on other chart types like bar charts or scatter plots
/// to provide additional statistical context.
///
/// # Color Handling
///
/// In error bar charts, colors can be assigned based on data categories or groups.
/// When color encoding is used, each error bar will be assigned a color from the
/// palette system to distinguish between different data series or categories.
/// When no explicit color encoding is provided, the `color` field in this struct
/// serves as the default color for all error bars. For grouped error bars,
/// different series are automatically assigned different colors from the palette
/// to distinguish them.
#[derive(Clone)]
pub struct MarkErrorBar {
    pub(crate) color: Option<SingleColor>,
    pub(crate) opacity: f64,
    pub(crate) stroke_width: f64,
    pub(crate) cap_length: f64,   // Add configurable cap length
    pub(crate) show_center: bool, // Add this field to control center visibility
}

impl MarkErrorBar {
    pub(crate) fn new() -> Self {
        Self {
            color: Some(SingleColor::new("black")),
            opacity: 1.0,
            stroke_width: 1.0,  // Default stroke width, 1.0 pixels
            cap_length: 3.0,    // Default cap length, 3.0 pixels
            show_center: false, // Don't show the center point by default
        }
    }
}

impl Default for MarkErrorBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkErrorBar {
    fn mark_type(&self) -> &'static str {
        "errorbar"
    }
}
