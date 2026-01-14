use crate::chart::Chart;
use crate::mark::point::MarkPoint;
use crate::visual::shape::PointShape;
use crate::visual::color::SingleColor;

/// Extension implementation for `Chart` to support Scatter Plots (MarkPoint).
/// 
/// This module provides the user-facing API to configure the Point mark.
/// Note that data-driven encodings (mapping columns to color/size) are handled 
/// by the encoder system, while these methods set the base/default properties.
impl Chart<MarkPoint> {
    
    /// Initializes a new `MarkPoint` (Scatter Plot) layer.
    /// 
    /// If a mark already exists, it keeps it; otherwise, it creates a new 
    /// default `MarkPoint`. This allows users to call configuration methods 
    /// in any order.
    pub fn mark_point(mut self) -> Self {
        if self.mark.is_none() {
            self.mark = Some(MarkPoint::default());
        }
        self
    }

    /// Set the default fill color for points.
    /// 
    /// # Arguments
    /// * `color` - An optional `SingleColor`. If `None`, points may have no fill.
    pub fn with_point_color(mut self, color: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.color = color;
        self.mark = Some(mark);
        self
    }

    /// Set the geometric shape for the points.
    /// 
    /// # Arguments
    /// * `shape` - A `PointShape` enum value (e.g., Circle, Square, Star).
    pub fn with_point_shape(mut self, shape: PointShape) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.shape = shape;
        self.mark = Some(mark);
        self
    }

    /// Set the default size (radius/half-width) for the points.
    pub fn with_point_size(mut self, size: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.size = size;
        self.mark = Some(mark);
        self
    }

    /// Set the opacity for the point layer.
    /// 
    /// # Arguments
    /// * `opacity` - A value from 0.0 to 1.0.
    pub fn with_point_opacity(mut self, opacity: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.opacity = opacity.clamp(0.0, 1.0);
        self.mark = Some(mark);
        self
    }

    /// Set the stroke (outline) color for the points.
    pub fn with_point_stroke(mut self, stroke: Option<SingleColor>) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.stroke = stroke;
        self.mark = Some(mark);
        self
    }

    /// Set the stroke width (thickness) for the points.
    pub fn with_point_stroke_width(mut self, width: f64) -> Self {
        let mut mark = self.mark.unwrap_or_default();
        mark.stroke_width = width;
        self.mark = Some(mark);
        self
    }
}
