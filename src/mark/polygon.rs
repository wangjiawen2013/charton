use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for closed, filled polygons.
///
/// Rows sharing a `path_group` value form one polygon. The renderer connects
/// them **in row order**, closes every ring and strokes its outline. When a
/// fill is present it paints the even-odd interior, which is what turns an
/// inner ring into a hole. This is the geometry for violin outlines, map
/// regions, hand-built filled ribbons and filled contour bands.
///
/// Its open counterpart is [`MarkPath`](crate::mark::path::MarkPath), which
/// stops at the last vertex and strokes the line instead of closing it.
///
/// Geographic maps use this same mark with
/// [`CoordSystem::Geo`](crate::coordinate::CoordSystem): projection belongs to
/// the coordinate system, not to the geometry.
#[derive(Clone, Debug)]
pub struct MarkPolygon {
    pub(crate) fill: SingleColor,
    pub(crate) opacity: f64,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f64,
    /// SVG-style dash pattern for the outline; empty draws a solid stroke.
    pub(crate) dash: Vec<f64>,
}

impl MarkPolygon {
    pub(crate) fn new() -> Self {
        Self {
            fill: SingleColor::new("gray"),
            opacity: 1.0,
            stroke: SingleColor::new("#333333"),
            stroke_width: 0.5,
            dash: vec![],
        }
    }

    /// Sets the fill color of the polygon. Accepts names like `"red"` or hex
    /// strings like `"#ff0000"`.
    pub fn with_fill(mut self, fill: impl Into<SingleColor>) -> Self {
        self.fill = fill.into();
        self
    }

    /// Sets the opacity of the polygon mark (0.0 to 1.0).
    pub const fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the stroke color of the polygon outline. Use `"none"` to disable it.
    pub fn with_stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the thickness of the polygon outline in pixels.
    pub const fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
    }

    /// Sets the outline dash pattern using the standard SVG `stroke-dasharray`
    /// rules.
    ///
    /// - `vec![]`: a solid outline.
    /// - `vec![step]`: equivalent to `vec![step, step]`.
    /// - `vec![dash, gap]`: explicit dash and gap lengths.
    ///
    /// Lengths are measured along the path's arc length, so the dash pattern
    /// stays even across segments on every backend.
    pub fn with_dash(mut self, dash: impl Into<Vec<f64>>) -> Self {
        self.dash = dash.into();
        self
    }
}

impl Default for MarkPolygon {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkPolygon {
    fn mark_type(&self) -> &'static str {
        "polygon"
    }

    fn stroke(&self) -> SingleColor {
        self.stroke
    }

    fn opacity(&self) -> f64 {
        self.opacity
    }
}
