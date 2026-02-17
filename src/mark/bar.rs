use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for bar charts.
///
/// The `MarkBar` struct defines the visual properties of rectangular bar elements.
/// It uses `Option<f64>` for width, spacing, and span to allow the coordinate system 
/// to provide "smart defaults" via `CoordLayout` if the user hasn't specified them.
#[derive(Debug, Clone)]
pub struct MarkBar {
    pub(crate) color: SingleColor,
    pub(crate) opacity: f64,
    pub(crate) stroke: Option<SingleColor>,
    pub(crate) stroke_width: Option<f64>,
    
    /// Dimensional override for bar width. 
    /// If None, the coordinate system's `default_bar_width` is used.
    pub(crate) width: Option<f64>,
    
    /// Dimensional override for spacing between bars in a group.
    /// If None, the coordinate system's `default_bar_spacing` is used.
    pub(crate) spacing: Option<f64>,
    
    /// Dimensional override for the total span of a bar group.
    /// If None, the coordinate system's `default_bar_span` is used.
    pub(crate) span: Option<f64>,
}

impl MarkBar {
    /// Creates a new `MarkBar` with default visual properties.
    /// 
    /// Dimensional fields (width, spacing, span) are initialized to `None` 
    /// to enable context-aware defaults from the coordinate system.
    pub(crate) fn new() -> Self {
        Self {
            color: SingleColor::new("steelblue"),
            opacity: 1.0,
            stroke: None,
            stroke_width: None,
            width: None,   // The maximal percentage of a bar's width relative to the tick interval. Defer to CoordLayout
            spacing: None, // The percentage of the space between bars within a group reltative to the bar width. Defer to CoordLayout
            span: None,    // The (width+spacing) of all bars in a group. Defer to CoordLayout
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the fill color of the bars. Accepts names like "red" or hex strings like "#ff0000".
    pub fn with_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the opacity of the bar mark (0.0 to 1.0).
    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the stroke color of the bars. Use "none" to disable the outline.
    pub fn with_stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = Some(stroke.into());
        self
    }

    /// Sets the thickness of the bar's outline in pixels.
    pub fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = Some(width);
        self
    }

    /// Manually sets the width of a bar. 
    /// 
    /// Providing a value here will override the coordinate system's default suggestion.
    pub fn with_width(mut self, width: f64) -> Self {
        self.width = Some(width.clamp(0.0, 1.0));
        self
    }

    /// Manually sets the relative spacing between bars within a group (0.0 to 1.0).
    pub fn with_spacing(mut self, spacing: f64) -> Self {
        self.spacing = Some(spacing.clamp(0.0, 1.0));
        self
    }

    /// Manually sets the total span of a bar group within a category (0.0 to 1.0).
    pub fn with_span(mut self, span: f64) -> Self {
        self.span = Some(span.clamp(0.0, 1.0));
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