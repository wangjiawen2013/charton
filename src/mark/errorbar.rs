use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for error bar charts.
///
/// The `MarkErrorBar` struct defines the visual properties of error bar elements.
/// It supports a fluent interface for configuring stroke appearance, cap dimensions,
/// and the visibility of center points for statistical uncertainty visualization.
#[derive(Clone)]
pub struct MarkErrorBar {
    pub(crate) color: SingleColor,
    pub(crate) opacity: f64,
    pub(crate) stroke_width: f64,
    pub(crate) cap_length: f64,
    pub(crate) show_center: bool,
    // --- Layout parameters for grouping (dodge) ---
    pub(crate) width: f64,   // Relative width of a error bar
    pub(crate) spacing: f64, // Proportional gap between bars in a group
    pub(crate) span: f64,    // Total category width allocated to the group
}

impl MarkErrorBar {
    pub(crate) fn new() -> Self {
        Self {
            // (0.000, 1.000, 0.961, 0.922), // #fff5eb
            color: SingleColor::new("black"),
            opacity: 1.0,
            stroke_width: 1.0,
            cap_length: 3.0,
            show_center: false,
            // Default layout values synced with MarkBar for seamless layering
            width: 0.5,
            spacing: 0.0,
            span: 0.7,
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the color of the error bar and its caps. Accepts "red", "#hex", etc.
    pub fn with_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the opacity of the error bar mark.
    ///
    /// Value should be between 0.0 (transparent) and 1.0 (opaque).
    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the thickness of the error bar lines.
    pub fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
    }

    /// Sets the length of the horizontal caps at the ends of the error bar.
    pub fn with_cap_length(mut self, length: f64) -> Self {
        self.cap_length = length;
        self
    }

    /// Determines whether to display a marker at the center (mean/median) of the error bar.
    pub fn with_center(mut self, show: bool) -> Self {
        self.show_center = show;
        self
    }

    /// Sets the relative width of the marks.
    pub fn with_width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }

    /// Sets the spacing between marks in a grouped layout.
    pub fn with_spacing(mut self, spacing: f64) -> Self {
        self.spacing = spacing.clamp(0.0, 1.0);
        self
    }

    /// Sets the total span of the group within a category.
    pub fn with_span(mut self, span: f64) -> Self {
        self.span = span.clamp(0.0, 1.0);
        self
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
