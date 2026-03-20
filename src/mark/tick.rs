use crate::mark::Mark;
use crate::visual::color::SingleColor;

/// Mark type for tick charts.
///
/// The `MarkTick` struct defines the visual properties of tick elements.
/// Ticks are short lines used to show distribution of data points along an axis,
/// similar to Altair's tick mark.
///
/// # Orientation
///
/// Ticks are always perpendicular to the x-axis by default (vertical lines).
/// To create horizontal ticks, simply swap your x and y encodings.
#[derive(Clone)]
pub struct MarkTick {
    pub(crate) color: SingleColor,
    pub(crate) stroke: SingleColor,
    pub(crate) thickness: f64,
    pub(crate) band_size: f64,
    pub(crate) opacity: f64,
}

impl MarkTick {
    pub(crate) fn new() -> Self {
        Self {
            color: SingleColor::new("black"),
            stroke: SingleColor::new("none"),
            thickness: 1.0,
            band_size: 7.0,
            opacity: 1.0,
        }
    }

    // --- Fluent Configuration Methods (Builder Pattern) ---

    /// Sets the fill color. Accepts "red", "#hex", etc.
    pub fn with_color(mut self, color: impl Into<SingleColor>) -> Self {
        self.color = color.into();
        self
    }

    /// Sets the stroke color. Accepts "red", "#hex", etc.
    pub fn with_stroke(mut self, stroke: impl Into<SingleColor>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Sets the thickness of the tick line.
    ///
    /// For vertical ticks, this is the width.
    /// For horizontal ticks (swapped axes), this is the height.
    ///
    /// # Example
    /// ```rust,ignore
    /// chart.mark_tick()?.configure_tick(|m| m.thickness(2.0))
    /// ```
    pub fn with_thickness(mut self, thickness: f64) -> Self {
        self.thickness = thickness.max(0.0);
        self
    }

    /// Sets the band size (length) of the tick.
    ///
    /// For vertical ticks, this controls the height.
    /// For horizontal ticks (swapped axes), this controls the width.
    ///
    /// # Example
    /// ```rust,ignore
    /// chart.mark_tick()?.configure_tick(|m| m.band_size(10.0))
    /// ```
    pub fn with_band_size(mut self, band_size: f64) -> Self {
        self.band_size = band_size.max(0.0);
        self
    }

    /// Sets the opacity of the tick mark.
    ///
    /// Value should be between 0.0 (transparent) and 1.0 (opaque).
    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }
}

impl Default for MarkTick {
    fn default() -> Self {
        Self::new()
    }
}

impl Mark for MarkTick {
    fn mark_type(&self) -> &'static str {
        "tick"
    }
}
