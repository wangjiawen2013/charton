use crate::mark::Mark;
use crate::visual::color::SingleColor;
use crate::visual::shape::PointShape;

/// Mark type for point/scatter charts.
///
/// The `MarkPoint` struct defines the visual properties of point elements. 
/// It supports a fluent interface for detailed configuration within chart layers.
#[derive(Clone)]
pub struct MarkPoint {
    pub(crate) color: Option<SingleColor>,
    pub(crate) shape: PointShape,
    pub(crate) size: f64,
    pub(crate) opacity: f64,
    pub(crate) stroke: Option<SingleColor>,
    pub(crate) stroke_width: f64,
}

impl MarkPoint {
    pub(crate) fn new() -> Self {
        Self {
            color: Some(SingleColor::new("black")),
            shape: PointShape::Circle,
            size: 3.0,
            opacity: 1.0,
            stroke: None,
            stroke_width: 0.0,
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the fill color of the point.
    /// 
    /// Accepts any type that can be converted into an `Option<SingleColor>`.
    pub fn color(mut self, color: impl Into<Option<SingleColor>>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the geometric shape of the point (e.g., Circle, Square, Triangle).
    pub fn shape(mut self, shape: PointShape) -> Self {
        self.shape = shape;
        self
    }

    /// Sets the size (radius or scale factor) of the point.
    pub fn size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    /// Sets the opacity of the point mark.
    /// 
    /// Value should be between 0.0 (transparent) and 1.0 (opaque).
    pub fn opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the stroke (outline) color of the point.
    pub fn stroke(mut self, stroke: impl Into<Option<SingleColor>>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the thickness of the point's outline.
    pub fn stroke_width(mut self, width: f64) -> Self {
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