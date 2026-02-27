use crate::scale::{Expansion, ResolvedScale, Scale, ScaleDomain};

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
#[derive(Debug, Clone)]
pub struct Color {
    // --- User Configuration (Intent/Inputs) ---
    /// The name of the data column used for color encoding.
    pub(crate) field: String,

    /// The desired scale transformation (e.g., Linear, Discrete, Log).
    pub(crate) scale_type: Option<Scale>,

    /// An explicit data domain for color mapping.
    pub(crate) domain: Option<ScaleDomain>,

    /// Rules for adding padding or buffer to the ends of the color scale domain.
    pub(crate) expansion: Option<Expansion>,

    // --- System Resolution (Result/Outputs) ---
    /// Stores the resolved scale instance. Using RwLock to support
    /// back-filling updates across multiple render calls.
    pub(crate) resolved_scale: ResolvedScale,
}

impl Color {
    /// Creates a new Color encoding for the specified data field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            scale_type: None,
            domain: None,
            expansion: None,
            resolved_scale: ResolvedScale::none(),
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
    pub fn with_expandsion(mut self, expansion: Expansion) -> Self {
        self.expansion = Some(expansion);
        self
    }
}

/// Convenience builder function to create a new Color encoding.
///
/// # Example
/// ```
/// let c = color("magnitude")
///     .with_scale(Scale::Linear)
///     .with_expansion(Expansion::default());
/// ```
pub fn color(field: &str) -> Color {
    Color::new(field)
}
