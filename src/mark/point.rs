use crate::mark::Mark;
use crate::visual::color::SingleColor;
use crate::visual::shape::PointShape;

/// Mark type for point/scatter charts.
///
/// The `MarkPoint` struct defines the visual properties of point elements. 
/// It supports a fluent interface for detailed configuration within chart layers.
#[derive(Clone)]
pub struct MarkPoint {
    pub(crate) color: SingleColor,
    pub(crate) shape: PointShape,
    pub(crate) size: f32,
    pub(crate) opacity: f32,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f32,
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
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the fill color. Accepts "red", "#hex", etc.
    pub fn color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the geometric shape of the point.
    /// 
    /// Accepts `PointShape` variants or string literals like "square".
    /// 
    /// # Example
    /// ```
    /// mark.shape("triangle"); // Automatic conversion
    /// ```
    pub fn shape(mut self, shape: impl Into<PointShape>) -> Self {
        self.shape = shape.into();
        self
    }

    /// Sets the size (radius or scale factor) of the point.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Sets the opacity of the point mark.
    /// 
    /// Value should be between 0.0 (transparent) and 1.0 (opaque).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the stroke color. Use "none" to disable.
    pub fn stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the thickness of the point's outline.
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
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