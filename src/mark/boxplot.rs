use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for box whisker charts
///
/// The `MarkBoxplot` struct defines the visual properties of box-and-whisker plot
/// elements used in statistical data visualization. It controls the appearance of
/// box plots including the box color, opacity, stroke, outlier styling, and layout
/// parameters for grouped displays.
///
/// Box plot marks are used to display the distribution of data based on a five-number
/// summary: minimum, first quartile, median, third quartile, and maximum. They are
/// particularly useful for identifying outliers and comparing distributions across
/// different categories.
///
/// # Color Handling
///
/// In box plots, colors can be assigned based on data categories or groups.
/// When color encoding is used, each box plot or group of box plots will be assigned
/// a color from the palette system to distinguish between different data series or
/// categories. When no explicit color encoding is provided, the `color` field in
/// this struct serves as the default fill color for all boxes. For grouped box plots,
/// different series are automatically assigned different colors from the palette
/// to distinguish them.
#[derive(Debug, Clone)]
pub struct MarkBoxplot {
    pub(crate) color: Option<SingleColor>,
    pub(crate) opacity: f64,
    pub(crate) stroke: Option<SingleColor>,
    pub(crate) stroke_width: f64,
    pub(crate) outlier_color: Option<SingleColor>,
    pub(crate) outlier_size: f64,
    pub(crate) width: f64,
    pub(crate) spacing: f64,
    pub(crate) span: f64,
}

impl MarkBoxplot {
    /// Create a new box whisker mark
    pub(crate) fn new() -> Self {
        Self {
            color: None,
            opacity: 1.0,
            stroke: Some(SingleColor::new("black")),
            stroke_width: 1.0,
            outlier_color: Some(SingleColor::new("black")),
            outlier_size: 3.0,
            width: 0.5,
            spacing: 0.2, // Gap(spacing*width) between dodged box elements in a group. 0.0-0.5 usually gives a beautiful layout.
            span: 0.7, // The total width of boxes and gaps in a position. 0.5-1.0 usually gives a beautiful layout.
        }
    }
}

impl Default for MarkBoxplot {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkBoxplot {
    fn mark_type(&self) -> &'static str {
        "boxplot"
    }
}
