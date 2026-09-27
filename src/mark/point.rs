use crate::mark::Mark;
use crate::visual::color::SingleColor;
use crate::visual::shape::PointShape;

/// Position adjustment methods for point marks on discrete axes.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum PointLayout {
    /// Points are placed exactly on the category center (may overlap).
    #[default]
    Standard,
    /// Points are randomly shifted horizontally within the allocated width.
    Jitter,
    /// Points are arranged using a force-directed layout to avoid overlap.
    Beeswarm,
    /// Points are spread by a density aware quasirandom sequence, so the
    /// outline of the cloud follows the shape of the data distribution.
    Quasirandom,
}

/// Strategy for deciding which side of the lane each point moves to in a
/// quasirandom layout.
///
/// The density estimate controls how far a point may move from the lane
/// center. The method controls how the many points that share a similar value
/// are paired, which changes the texture of the cloud.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QuasirandomMethod {
    /// Pairs nearby points with a low discrepancy sequence. This produces the
    /// smooth, symmetric violin outline that the layout is known for.
    #[default]
    Tukey,
    /// Pairs nearby points with a deterministic random-like sequence. The
    /// outline is noisier and never shows a repeating band.
    Pseudorandom,
}

/// Implements conversion from string slices for a more ergonomic Fluent API.
///
/// Every layout has exactly one name. An unrecognized name falls back to the
/// default layout, so a configuration call never fails. Callers that need to
/// catch misspellings should match on the enum instead of passing a string.
impl From<&str> for PointLayout {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "jitter" => PointLayout::Jitter,
            "beeswarm" => PointLayout::Beeswarm,
            "quasirandom" => PointLayout::Quasirandom,
            _ => PointLayout::Standard,
        }
    }
}

impl From<&str> for QuasirandomMethod {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pseudorandom" => QuasirandomMethod::Pseudorandom,
            _ => QuasirandomMethod::Tukey,
        }
    }
}

/// Mark type for point/scatter charts.
///
/// The `MarkPoint` struct defines the visual properties of point elements.
/// It supports a fluent interface for detailed configuration within chart layers.
#[derive(Clone)]
pub struct MarkPoint {
    pub(crate) color: SingleColor,
    pub(crate) shape: PointShape,
    pub(crate) size: f64,
    pub(crate) opacity: f64,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f64,

    // --- Layout strategy ---
    /// The physical arrangement strategy (Standard, Jitter, Beeswarm, or
    /// Quasirandom).
    pub(crate) layout: PointLayout,
    /// The pairing strategy used when the layout is Quasirandom.
    pub(crate) quasirandom_method: QuasirandomMethod,

    // --- Layout parameters for grouping (dodge) ---
    /// Relative width of a group/lane. In Jitter/Beeswarm mode,
    /// points stay within this boundary.
    pub(crate) width: f64,
    /// Proportional gap between groups at an axis position.
    pub(crate) spacing: f64,
    /// Total width of all groups combined at the axis position.
    pub(crate) span: f64,
}

impl MarkPoint {
    pub(crate) fn new() -> Self {
        Self {
            color: SingleColor::new("black"),
            shape: PointShape::Circle,
            size: 3.0,
            opacity: 1.0,
            stroke: SingleColor::new("none"),
            stroke_width: 0.0,
            layout: PointLayout::Standard,
            quasirandom_method: QuasirandomMethod::Tukey,
            width: 0.5,
            spacing: 0.2,
            span: 0.7,
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the fill color. Accepts "red", "#hex", etc.
    pub fn with_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the geometric shape of the point.
    ///
    /// Accepts `PointShape` variants or string literals like "square".
    ///
    pub fn with_shape(mut self, shape: impl Into<PointShape>) -> Self {
        self.shape = shape.into();
        self
    }

    /// Sets the size (radius or scale factor) of the point.
    pub const fn with_size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    /// Sets the opacity of the point mark.
    ///
    /// Value should be between 0.0 (transparent) and 1.0 (opaque).
    pub const fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the stroke color. Use "none" to disable.
    pub fn with_stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the thickness of the point's outline.
    pub const fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
    }

    /// Sets the layout strategy for point marks.
    ///
    /// Accepts `PointLayout` variants or string literals like "jitter".
    pub fn with_layout(mut self, layout: impl Into<PointLayout>) -> Self {
        self.layout = layout.into();
        self
    }

    /// Sets the pairing strategy for a quasirandom layout.
    ///
    /// Accepts `QuasirandomMethod` variants or string literals like
    /// "pseudorandom". It has no effect on other layouts.
    pub fn with_quasirandom_method(mut self, method: impl Into<QuasirandomMethod>) -> Self {
        self.quasirandom_method = method.into();
        self
    }

    /// Sets the relative width of the marks.
    pub const fn with_width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }

    /// Sets the spacing between marks in a grouped layout.
    pub const fn with_spacing(mut self, spacing: f64) -> Self {
        self.spacing = spacing.clamp(0.0, 1.0);
        self
    }

    /// Sets the total span of the group within a category.
    pub const fn with_span(mut self, span: f64) -> Self {
        self.span = span.clamp(0.0, 1.0);
        self
    }
}

impl Default for MarkPoint {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkPoint {
    fn mark_type(&self) -> &'static str {
        "point"
    }
}
