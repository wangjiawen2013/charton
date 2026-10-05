//! The 2-D density mark: a bivariate kernel density drawn as a grid of cells.
//!
//! The two columns come from `x` and `y`; the joint estimate is written to a
//! generated `density` column and drawn as a heatmap.
//!
//! # Example
//!
//! ```rust,ignore
//! chart!(iris)?
//!     .mark_density_2d()?
//!     .configure_density_2d(|d| d.with_grid_size(60).with_padding(0.2))
//!     .encode((alt::x("sepal_length"), alt::y("petal_length")))?
//!     .save("density_heatmap.svg")?;
//! ```

use crate::mark::Mark;
use crate::transform::density_transform::BandwidthType;
use crate::visual::color::SingleColor;

/// Configuration for
/// [`Chart::mark_density_2d`](crate::chart::Chart::mark_density_2d).
///
/// The visual style is forwarded to the rectangle geometry; the cells are
/// normally coloured by their density through the theme's colour map, so the
/// fill here is only a fallback.
#[derive(Debug, Clone)]
pub struct MarkDensity2D {
    // --- Visual style (forwarded to the rectangle geometry) ---
    pub(crate) color: SingleColor,
    pub(crate) opacity: f64,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f64,

    // --- Recipe parameters ---
    pub(crate) bandwidth: BandwidthType,
    pub(crate) grid: usize,
    pub(crate) padding: f64,

    // --- Resolved input columns ---
    pub(crate) inputs_resolved: bool,
    pub(crate) x_field: Option<String>,
    pub(crate) y_field: Option<String>,
}

impl MarkDensity2D {
    pub(crate) fn new() -> Self {
        Self {
            color: SingleColor::new("black"),
            opacity: 1.0,
            stroke: SingleColor::new("white"),
            stroke_width: 0.0,

            bandwidth: BandwidthType::Scott,
            grid: 50,
            padding: 0.1,

            inputs_resolved: false,
            x_field: None,
            y_field: None,
        }
    }

    /// Sets the fallback fill colour of a cell. Cells are coloured by density,
    /// so this is only used when colour mapping is unavailable.
    pub fn with_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the cell opacity (clamped to `0.0..=1.0`).
    pub const fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the cell outline colour. Use `"none"` to disable it.
    pub fn with_stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the cell outline thickness. `0.0` (the default) hides the grid.
    pub const fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
    }

    /// Chooses how the kernel bandwidth is picked (Scott, Silverman, or fixed).
    pub const fn with_bandwidth(mut self, bandwidth: BandwidthType) -> Self {
        self.bandwidth = bandwidth;
        self
    }

    /// Sets the number of grid nodes per axis. The heatmap uses exactly this
    /// many cells per axis, so a finer grid gives a smoother picture.
    pub fn with_grid_size(mut self, grid: usize) -> Self {
        self.grid = grid.max(2);
        self
    }

    /// Sets how far the grid is widened past the data, as a fraction.
    pub const fn with_padding(mut self, padding: f64) -> Self {
        self.padding = padding.max(0.0);
        self
    }
}

impl Default for MarkDensity2D {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkDensity2D {
    fn mark_type(&self) -> &'static str {
        "density_2d"
    }

    fn stroke(&self) -> SingleColor {
        self.stroke
    }

    fn opacity(&self) -> f64 {
        self.opacity
    }

    fn as_density_2d(&self) -> Option<&MarkDensity2D> {
        Some(self)
    }

    fn as_density_2d_mut(&mut self) -> Option<&mut MarkDensity2D> {
        Some(self)
    }
}
