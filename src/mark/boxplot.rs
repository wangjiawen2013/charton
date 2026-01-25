use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for box whisker charts.
///
/// The `MarkBoxplot` struct defines the visual properties of box-and-whisker plot elements.
/// It supports a fluent interface for configuring the box appearance, outlier styling,
/// and statistical layout parameters like spacing and span.
#[derive(Debug, Clone)]
pub struct MarkBoxplot {
    pub(crate) color: SingleColor,
    pub(crate) opacity: f32,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f32,
    pub(crate) outlier_color: SingleColor,
    pub(crate) outlier_size: f32,
    pub(crate) width: f32,
    pub(crate) spacing: f32,
    pub(crate) span: f32,
}

impl MarkBoxplot {
    pub(crate) fn new() -> Self {
        Self {
            // (0.125, 0.996, 0.902, 0.808), // #fee6ce
            color: SingleColor::new("none"),
            opacity: 1.0,
            stroke: SingleColor::new("black"),
            stroke_width: 1.0,
            outlier_color: SingleColor::new("black"),
            outlier_size: 3.0,
            width: 0.5,
            spacing: 0.2,
            span: 0.7,
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the fill color of the boxes. Accepts "red", "#hex", etc.
    pub fn with_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the opacity of the boxplot mark.
    /// 
    /// Value should be between 0.0 (transparent) and 1.0 (opaque).
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the stroke color of the boxes and whiskers. Use "none" to disable.
    pub fn with_stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the thickness of the boxplot's lines.
    pub fn with_stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    /// Sets the color of the outlier points.
    pub fn with_outlier_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.outlier_color = color.into();
        self
    }

    /// Sets the size of the outlier points.
    pub fn with_outlier_size(mut self, size: f32) -> Self {
        self.outlier_size = size;
        self
    }

    /// Sets the maximal width of the boxes.
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets the relative spacing between boxes in a group.
    /// 
    /// Value is clamped between 0.0 and 1.0.
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing.clamp(0.0, 1.0);
        self
    }

    /// Sets the total span allocated for a box group.
    /// 
    /// Value is clamped between 0.0 and 1.0.
    pub fn with_span(mut self, span: f32) -> Self {
        self.span = span.clamp(0.0, 1.0);
        self
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