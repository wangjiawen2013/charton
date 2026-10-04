use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for path and polygon charts — the general "connect the dots"
/// geometry.
///
/// Rows that share a `path_group` value form one shape. The renderer connects
/// them **in row order**: it never sorts and never inserts points. The one flag
/// that changes the geometry is `closed`:
///
/// * `closed = true` (used by `mark_polygon` / `mark_geoshape`) draws
///   `M p0 L p1 ... Z` — the `Z` adds the edge back to the first point and the
///   interior is filled. This is a region: a violin outline, a map polygon, a
///   filled contour band.
/// * `closed = false` (used by `mark_path`) draws `M p0 L p1 ...` and stops at
///   the last point. There is no closing edge and no fill. This is a line:
///   a trajectory, a contour line, a network edge.
///
/// The two are not interchangeable. Feeding an **open** curve to the closed form
/// adds a spurious edge from the last point back to the first; feeding a line
/// loop (for example a closed contour) to the closed form would fill it,
/// whereas you usually want just the stroke. The closed flag *is* the
/// difference between "outline" and "trajectory".
///
/// Both forms take an optional dash pattern through `with_dash`. It is measured
/// along the path's arc length, so a dashed contour or a dashed polygon outline
/// stays even across segments on every backend, CPU and GPU alike.
#[derive(Clone, Debug)]
pub struct MarkGeoPath {
    pub(crate) fill: SingleColor,
    pub(crate) opacity: f64,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f64,
    /// SVG-style dash pattern for the outline; empty draws a solid stroke.
    pub(crate) dash: Vec<f64>,
    /// `true` closes the vertex loop and fills it; `false` draws an open line.
    pub(crate) closed: bool,
}

impl MarkGeoPath {
    pub(crate) fn new() -> Self {
        Self {
            fill: SingleColor::new("gray"),
            opacity: 1.0,
            stroke: SingleColor::new("#333333"),
            stroke_width: 0.5,
            dash: vec![],
            closed: true,
        }
    }

    /// Closes (`true`) or opens (`false`) the vertex loop.
    ///
    /// A closed path is filled; an open path is stroked only. `mark_path` sets
    /// this to `false`, `mark_polygon` to `true`.
    pub const fn with_closed(mut self, closed: bool) -> Self {
        self.closed = closed;
        self
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

    /// Sets the outline dash pattern using the standard SVG
    /// `stroke-dasharray` rules.
    ///
    /// An empty pattern (the default) draws a solid outline. `[dash, gap]`
    /// alternates a dash and a gap of the given lengths; an odd number of
    /// entries is repeated once so the pattern always alternates. Lengths are
    /// measured along the path, so a dashed contour reads as a broken line.
    pub fn with_dash(mut self, dash: impl Into<Vec<f64>>) -> Self {
        self.dash = dash.into();
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
