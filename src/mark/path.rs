use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for open paths.
///
/// Rows sharing a `path_group` value form one path. The renderer connects them
/// **in row order** — it never sorts and never inserts points — and leaves the
/// shape open: it stops at the last vertex and strokes the result. This is the
/// geometry for trajectories, contour lines, network edges and parallel
/// coordinates.
///
/// Its closed counterpart is [`MarkPolygon`](crate::mark::polygon::MarkPolygon),
/// which joins the last vertex back to the first and, when a fill is present,
/// paints the even-odd interior.
#[derive(Clone, Debug)]
pub struct MarkPath {
    pub(crate) opacity: f64,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f64,
    /// SVG-style dash pattern; empty draws a solid stroke.
    pub(crate) dash: Vec<f64>,
}

impl MarkPath {
    pub(crate) fn new() -> Self {
        Self {
            opacity: 1.0,
            stroke: SingleColor::new("black"),
            stroke_width: 1.0,
            dash: vec![],
        }
    }

    /// Sets the stroke color of the path. Accepts names like `"red"` or hex
    /// strings like `"#ff0000"`.
    pub fn with_stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the thickness of the path stroke in pixels.
    pub const fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
    }

    /// Sets the opacity of the path mark (0.0 to 1.0).
    pub const fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the dash pattern using the standard SVG `stroke-dasharray` rules.
    ///
    /// - `vec![]`: a solid line.
    /// - `vec![step]`: equivalent to `vec![step, step]`.
    /// - `vec![dash, gap]`: explicit dash and gap lengths.
    ///
    /// Lengths are measured along the path's arc length, so a dashed contour
    /// stays even across segments on every backend.
    pub fn with_dash(mut self, dash: impl Into<Vec<f64>>) -> Self {
        self.dash = dash.into();
        self
    }
}

impl Default for MarkPath {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkPath {
    fn mark_type(&self) -> &'static str {
        "path"
    }

    fn stroke(&self) -> SingleColor {
        self.stroke
    }

    fn opacity(&self) -> f64 {
        self.opacity
    }
}
