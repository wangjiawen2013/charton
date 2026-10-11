use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for rectangle/heatmap charts.
///
/// The `MarkRect` struct defines the visual properties of rectangular elements.
/// It supports a fluent interface for configuring fill color, opacity,
/// and boundary stroke properties.
#[derive(Clone)]
pub struct MarkRect {
    pub(crate) color: SingleColor,
    pub(crate) opacity: f64,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f64,
    /// Extra device pixels added to every side of a cell. A heatmap's tiles
    /// meet edge to edge, and anti-aliasing then leaves a hairline seam;
    /// a small positive padding closes it. Negative values open a gap.
    pub(crate) cell_padding: f64,
}

impl MarkRect {
    pub(crate) fn new() -> Self {
        Self {
            // (0.500, 0.992, 0.553, 0.235), // #fd8d3c
            color: SingleColor::new("black"),
            opacity: 1.0,
            // No outline by default; a real colour at width 0 is contradictory
            // and lets a rasterizer leave a hairline between tiles.
            stroke: SingleColor::none(),
            stroke_width: 0.0,
            // Enough to overlap the anti-aliased edge of the neighbouring tile.
            cell_padding: 0.5,
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the fill color of the rectangle. Accepts "red", "#hex", etc.
    pub fn with_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the opacity of the rectangle mark.
    ///
    /// Value should be between 0.0 (transparent) and 1.0 (opaque).
    pub const fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the stroke color of the rectangle boundary. Use "none" to disable.
    pub fn with_stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the thickness of the rectangle's outline.
    pub const fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
    }

    /// Grows each cell by `padding` device pixels on every side.
    ///
    /// Use a small positive value to close the hairline seams that
    /// anti-aliasing leaves between heatmap tiles, or a negative value to open
    /// a deliberate gap. The default is `0.5`.
    pub const fn with_cell_padding(mut self, padding: f64) -> Self {
        self.cell_padding = padding;
        self
    }
}

impl Default for MarkRect {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkRect {
    fn mark_type(&self) -> &'static str {
        "rect"
    }
}
