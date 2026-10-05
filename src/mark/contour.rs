//! The contour mark: iso-lines of a scalar field, drawn as open paths.
//!
//! `x` and `y` come from the encodings and the scalar field `z` is named
//! explicitly (the crate has no `z` channel). The mark runs `transform_contour`
//! (marching squares) and draws the lines with the path renderer.
//!
//! # Example
//!
//! ```rust,ignore
//! chart!(x, y, z)?
//!     .mark_contour("z")?
//!     .encode((alt::x("x"), alt::y("y")))?
//!     .save("contour.svg")?;
//! ```

use crate::mark::Mark;
use crate::transform::contour_transform::ContourLevels;
use crate::visual::color::SingleColor;

/// Configuration for [`Chart::mark_contour`](crate::chart::Chart::mark_contour).
///
/// By default the iso-lines are coloured by their `level` (the multi-colour
/// gallery contour). Use [`with_color_by_level(false)`](MarkContour::with_color_by_level)
/// for a single-colour outline, or set a `color` encoding yourself to override.
#[derive(Debug, Clone)]
pub struct MarkContour {
    // --- Visual style ---
    pub(crate) stroke: SingleColor,
    pub(crate) opacity: f64,
    pub(crate) stroke_width: f64,
    pub(crate) dash: Vec<f64>,

    // --- Recipe parameters ---
    pub(crate) levels: ContourLevels,
    pub(crate) color_by_level: bool,

    // --- Resolved input columns ---
    //
    // `z` is supplied to `mark_contour` explicitly. `x`/`y` are read from the
    // encodings on the first transform and cached here, because a faceted
    // re-run sees an encoding that already points at the generated columns.
    pub(crate) inputs_resolved: bool,
    pub(crate) x_field: Option<String>,
    pub(crate) y_field: Option<String>,
    pub(crate) z_field: Option<String>,
}

impl MarkContour {
    pub(crate) fn new(z: impl Into<String>) -> Self {
        Self {
            stroke: SingleColor::new("black"),
            opacity: 1.0,
            stroke_width: 1.0,
            dash: Vec::new(),

            levels: ContourLevels::Count(8),
            color_by_level: true,

            inputs_resolved: false,
            x_field: None,
            y_field: None,
            z_field: Some(z.into()),
        }
    }

    /// Sets the outline colour (used when colour-by-level is off).
    pub fn with_stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the outline opacity (clamped to `0.0..=1.0`).
    pub const fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the outline thickness.
    pub const fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
    }

    /// Sets the outline dash pattern (an empty pattern is solid).
    pub fn with_dash(mut self, dash: impl Into<Vec<f64>>) -> Self {
        self.dash = dash.into();
        self
    }

    /// Draws `n` evenly spaced levels strictly inside the data range.
    pub fn with_levels(mut self, n: usize) -> Self {
        self.levels = ContourLevels::Count(n.max(1));
        self
    }

    /// Draws exactly these level values.
    pub fn with_levels_values(mut self, values: impl Into<Vec<f64>>) -> Self {
        self.levels = ContourLevels::Values(values.into());
        self
    }

    /// Colours each line by its level when `true` (the default).
    ///
    /// This adds a continuous `color` scale over the generated `level` column,
    /// which the theme's colour map drives. Set to `false` for a single-colour
    /// outline. A `color` encoding supplied by the caller is never overwritten.
    pub const fn with_color_by_level(mut self, color_by_level: bool) -> Self {
        self.color_by_level = color_by_level;
        self
    }
}

impl Default for MarkContour {
    fn default() -> Self {
        Self {
            stroke: SingleColor::new("black"),
            opacity: 1.0,
            stroke_width: 1.0,
            dash: Vec::new(),
            levels: ContourLevels::Count(8),
            color_by_level: true,
            inputs_resolved: false,
            x_field: None,
            y_field: None,
            z_field: None,
        }
    }
}

impl Mark for MarkContour {
    fn mark_type(&self) -> &'static str {
        "contour"
    }

    fn stroke(&self) -> SingleColor {
        self.stroke
    }

    fn opacity(&self) -> f64 {
        self.opacity
    }

    fn as_contour(&self) -> Option<&MarkContour> {
        Some(self)
    }

    fn as_contour_mut(&mut self) -> Option<&mut MarkContour> {
        Some(self)
    }
}
