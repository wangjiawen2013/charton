use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for geographic path (polygon) charts.
///
/// `MarkGeoPath` renders closed polygons suitable for map boundaries,
/// administrative regions, and any spatial area data in long-form format.
///
/// Each polygon is defined by a group of (longitude, latitude) vertices
/// sharing the same `PathGroup` value. The renderer connects them in
/// row order and closes the path automatically.
#[derive(Clone, Debug)]
pub struct MarkGeoPath {
    pub(crate) fill: SingleColor,
    pub(crate) opacity: f64,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f64,
}

impl MarkGeoPath {
    pub(crate) fn new() -> Self {
        Self {
            fill: SingleColor::new("gray"),
            opacity: 1.0,
            stroke: SingleColor::new("#333333"),
            stroke_width: 0.5,
        }
    }

    /// Sets the fill color of the geographic region.
    pub fn with_fill(mut self, color: impl Into<SingleColor>) -> Self {
        self.fill = color.into();
        self
    }

    /// Sets the opacity of the geographic region fill.
    pub const fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the stroke color for polygon boundaries.
    pub fn with_stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the stroke width for polygon boundaries.
    pub const fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
    }
}

impl Default for MarkGeoPath {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkGeoPath {
    fn mark_type(&self) -> &'static str {
        "geo_path"
    }

    fn stroke(&self) -> SingleColor {
        self.stroke
    }

    fn opacity(&self) -> f64 {
        self.opacity
    }
}
