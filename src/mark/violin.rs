//! The violin mark: a density estimate drawn as a symmetric polygon.
//!
//! The value is read from `y`, an optional category from `x`, and an optional
//! group from `color`. The mark runs `transform_density` (one curve per group)
//! and draws each curve as a symmetric `transform_band` polygon.
//!
//! # Example
//!
//! ```rust,ignore
//! chart!(penguins)?
//!     .mark_violin()?
//!     .configure_violin(|v| v.with_split(false))
//!     .encode((alt::x("Sex"), alt::y("Body Mass (g)"), alt::color("Species")))?
//!     .save("violin.svg")?;
//! ```

use crate::mark::Mark;
use crate::transform::band_transform::BandScale;
use crate::transform::density_transform::BandwidthType;
use crate::visual::color::SingleColor;

/// Configuration for [`Chart::mark_violin`](crate::chart::Chart::mark_violin).
///
/// A violin is the density of a numeric column drawn as a symmetric band. The
/// defaults mirror ggplot2's `geom_violin`: a trimmed (data-bounded) density, a
/// Gaussian kernel with Scott's rule, and a per-group width scale.
#[derive(Debug, Clone)]
pub struct MarkViolin {
    // --- Visual style ---
    pub(crate) color: SingleColor,
    pub(crate) opacity: f64,
    pub(crate) stroke: SingleColor,
    pub(crate) stroke_width: f64,

    // --- Recipe parameters ---
    pub(crate) bandwidth: BandwidthType,
    pub(crate) trim: bool,
    pub(crate) split: bool,
    pub(crate) scale: BandScale,
    pub(crate) width: f64,
    pub(crate) span: f64,

    // --- Resolved input columns ---
    //
    // Populated from the encodings on the first transform. A faceted chart
    // re-runs the recipe on each panel subset, at which point the encoding
    // points at the generated columns; these stored names are what let the
    // re-run find the original data.
    pub(crate) inputs_resolved: bool,
    pub(crate) value_field: Option<String>,
    pub(crate) center_field: Option<String>,
    pub(crate) group_field: Option<String>,
}

impl MarkViolin {
    pub(crate) fn new() -> Self {
        Self {
            color: SingleColor::new("#aed6f1"),
            opacity: 0.8,
            stroke: SingleColor::new("#2c3e50"),
            stroke_width: 1.0,

            bandwidth: BandwidthType::Scott,
            trim: true,
            split: false,
            scale: BandScale::PerGroup,
            width: 0.5,
            span: 0.7,

            inputs_resolved: false,
            value_field: None,
            center_field: None,
            group_field: None,
        }
    }

    /// Sets the fill colour of the violin body. Accepts `"red"`, `"#hex"`, ...
    pub fn with_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the opacity of the violin body (clamped to `0.0..=1.0`).
    pub const fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets the outline colour. Use `"none"` to disable the outline.
    pub fn with_stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the outline thickness.
    pub const fn with_stroke_width(mut self, width: f64) -> Self {
        self.stroke_width = width;
        self
    }

    /// Chooses how the kernel bandwidth is picked (Scott, Silverman, or fixed).
    /// A wider bandwidth gives a smoother violin.
    pub const fn with_bandwidth(mut self, bandwidth: BandwidthType) -> Self {
        self.bandwidth = bandwidth;
        self
    }

    /// Trims each density to its group's observed range.
    ///
    /// `true` (the default) makes a violin end at the data extremes instead of
    /// fading into a thin near-zero tail. Set `false` for the smooth tails of a
    /// density plot.
    pub const fn with_trim(mut self, trim: bool) -> Self {
        self.trim = trim;
        self
    }

    /// Draws two groups as the left and right halves of one violin.
    ///
    /// Requires a `color` encoding; the first group grows right, the second
    /// left. Without a colour group this has no effect.
    pub const fn with_split(mut self, split: bool) -> Self {
        self.split = split;
        self
    }

    /// Chooses the width scaling rule for side-by-side violins
    /// (`PerGroup` / `Global` / `Raw`). See [`BandScale`].
    pub const fn with_scale(mut self, scale: BandScale) -> Self {
        self.scale = scale;
        self
    }

    /// Sets the maximum width of a single violin, in category steps.
    pub const fn with_width(mut self, width: f64) -> Self {
        self.width = width.clamp(0.0, 1.0);
        self
    }

    /// Sets the total width of a category's violin group, in category steps.
    pub const fn with_span(mut self, span: f64) -> Self {
        self.span = span.clamp(0.0, 1.0);
        self
    }
}

impl Default for MarkViolin {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkViolin {
    fn mark_type(&self) -> &'static str {
        "violin"
    }

    fn stroke(&self) -> SingleColor {
        self.stroke
    }

    fn opacity(&self) -> f64 {
        self.opacity
    }

    fn as_violin(&self) -> Option<&MarkViolin> {
        Some(self)
    }

    fn as_violin_mut(&mut self) -> Option<&mut MarkViolin> {
        Some(self)
    }
}
