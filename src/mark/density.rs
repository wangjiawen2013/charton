//! The density mark: a one-dimensional kernel density estimate drawn as a
//! smooth area.
//!
//! The value is read from `x` and an optional group from `color`; the estimate
//! is written to a generated `density` column and drawn as an area.
//!
//! # Example
//!
//! ```rust,ignore
//! chart!(iris)?
//!     .mark_density()?
//!     .configure_density(|d| d.with_opacity(0.5))
//!     .encode((alt::x("sepal_length"), alt::color("species")))?
//!     .save("density.svg")?;
//! ```

use crate::mark::Mark;
use crate::transform::density_transform::{BandwidthType, KernelType};
use crate::visual::color::SingleColor;

/// Configuration for [`Chart::mark_density`](crate::chart::Chart::mark_density).
///
/// Defaults follow `transform_density`: an untrimmed density (the tails fade
/// out past the last observation, the `geom_density` look), Scott's rule and a
/// Gaussian kernel.
#[derive(Debug, Clone)]
pub struct MarkDensity {
    // --- Visual style (forwarded to the area geometry) ---
    pub(crate) color: SingleColor,
    pub(crate) opacity: f64,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f64,
    pub(crate) dash: Vec<f64>,

    // --- Recipe parameters ---
    pub(crate) bandwidth: BandwidthType,
    pub(crate) kernel: KernelType,
    pub(crate) trim: bool,
    pub(crate) counts: bool,
    pub(crate) cumulative: bool,

    // --- Resolved input columns ---
    //
    // Cached on the first transform so a faceted re-run (when the encoding no
    // longer names the original column) can still find the value and group.
    pub(crate) inputs_resolved: bool,
    pub(crate) value_field: Option<String>,
    pub(crate) group_field: Option<String>,
}

impl MarkDensity {
    pub(crate) fn new() -> Self {
        Self {
            color: SingleColor::new("gray"),
            opacity: 1.0,
            stroke: SingleColor::new("none"),
            stroke_width: 1.0,
            dash: Vec::new(),

            bandwidth: BandwidthType::Scott,
            kernel: KernelType::Normal,
            trim: false,
            counts: false,
            cumulative: false,

            inputs_resolved: false,
            value_field: None,
            group_field: None,
        }
    }

    /// Sets the fill colour of the curve. Accepts `"red"`, `"#hex"`, ...
    pub fn with_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the fill opacity (clamped to `0.0..=1.0`). Useful when several
    /// curves overlap.
    pub const fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the outline colour. Use `"none"` (the default) for no outline.
    pub fn with_stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
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

    /// Chooses how the kernel bandwidth is picked (Scott, Silverman, or fixed).
    pub const fn with_bandwidth(mut self, bandwidth: BandwidthType) -> Self {
        self.bandwidth = bandwidth;
        self
    }

    /// Chooses the smoothing kernel.
    pub fn with_kernel(mut self, kernel: impl Into<KernelType>) -> Self {
        self.kernel = kernel.into();
        self
    }

    /// Trims each curve to its group's observed range.
    ///
    /// The default `false` keeps the smooth tails (`geom_density`); `true` is
    /// the violin-like behaviour.
    pub const fn with_trim(mut self, trim: bool) -> Self {
        self.trim = trim;
        self
    }

    /// Emits smoothed counts instead of a probability density.
    pub const fn with_counts(mut self, counts: bool) -> Self {
        self.counts = counts;
        self
    }

    /// Emits the cumulative distribution instead of the density.
    pub const fn with_cumulative(mut self, cumulative: bool) -> Self {
        self.cumulative = cumulative;
        self
    }
}

impl Default for MarkDensity {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkDensity {
    fn mark_type(&self) -> &'static str {
        "density"
    }

    fn stroke(&self) -> SingleColor {
        self.stroke
    }

    fn opacity(&self) -> f64 {
        self.opacity
    }

    fn as_density(&self) -> Option<&MarkDensity> {
        Some(self)
    }

    fn as_density_mut(&mut self) -> Option<&mut MarkDensity> {
        Some(self)
    }
}
