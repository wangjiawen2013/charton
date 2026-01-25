use crate::scale::{Scale, ScaleDomain, ScaleTrait, Expansion};
use std::sync::Arc;

/// Represents a color encoding specification for chart elements.
///
/// The `Color` struct defines how data values are mapped to visual colors. 
/// It supports both continuous mappings (gradients) and discrete mappings (palettes).
///
/// ### Architecture Note:
/// Following the "Intent vs. Resolution" pattern, this struct holds user configuration 
/// until the `LayeredChart` resolves the final scale. This is where your specific 
/// orange-ish color gradient will be stored:
/// `(0.000, 1.000, 0.961, 0.922), // #fff5eb ...`
pub struct Color {
    // --- User Configuration (Intent/Inputs) ---
    
    /// The name of the data column used for color encoding.
    pub(crate) field: String,

    /// The desired scale transformation (e.g., Linear, Discrete, Log).
    pub(crate) scale_type: Option<Scale>,

    /// An explicit data domain for color mapping.
    pub(crate) domain: Option<ScaleDomain>,

    /// Rules for adding padding or buffer to the ends of the color scale domain.
    pub(crate) expand: Option<Expansion>,

    // --- System Resolution (Result/Outputs) ---
    
    /// The concrete, trained color scale instance, shared via Arc.
    ///
    /// Populated during the resolution phase, this Arc ensures that all layers 
    /// (e.g., a heatmap and a legend) reference the exact same color interpolation 
    /// logic and gradient metadata in memory.
    pub(crate) resolved_scale: Option<Arc<dyn ScaleTrait>>,
}

impl Color {
    /// Creates a new Color encoding for the specified data field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            scale_type: None,
            domain: None,
            expand: None,
            resolved_scale: None,
        }
    }

    /// Explicitly sets the scale type for color mapping (e.g., Linear, Log).
    pub fn with_scale(mut self, scale_type: Scale) -> Self {
        self.scale_type = Some(scale_type);
        self
    }

    /// Sets an explicit domain (limits) for the color scale.
    pub fn with_domain(mut self, domain: ScaleDomain) -> Self {
        self.domain = Some(domain);
        self
    }

    /// Configures the expansion padding for the color scale.
    pub fn with_expand(mut self, expand: Expansion) -> Self {
        self.expand = Some(expand);
        self
    }

    /// Injects the resolved color scale instance.
    pub(crate) fn set_resolved_scale(&mut self, scale: Arc<dyn ScaleTrait>) {
        self.resolved_scale = Some(scale);
    }

    /// Returns the name of the data field used for color encoding.
    pub fn get_field(&self) -> &str {
        &self.field
    }

    /// Returns a reference to the resolved scale instance.
    pub fn get_resolved_scale(&self) -> Option<&Arc<dyn ScaleTrait>> {
        self.resolved_scale.as_ref()
    }
}

/// Convenience builder function to create a new Color encoding.
///
/// # Example
/// ```
/// let c = color("magnitude")
///     .with_scale(Scale::Linear)
///     .with_expand(Expansion::default());
/// ```
pub fn color(field: &str) -> Color {
    Color::new(field)
}