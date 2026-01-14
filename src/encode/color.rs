use crate::scale::{Scale, ScaleDomain};

/// Represents a color encoding for chart elements.
///
/// The `Color` struct defines how data values should be mapped to colors 
/// in a visualization. It supports both continuous scales (e.g., gradients 
/// for price) and discrete scales (e.g., distinct colors for categories).
#[derive(Debug, Clone)]
pub struct Color {
    /// The name of the data column to use for color encoding.
    pub field: String,

    /// The scale type for color mapping (Linear, Log, Discrete, etc.).
    /// If None, the system will automatically infer the scale based on data type.
    pub scale: Option<Scale>,

    /// The resolved data domain used for mapping.
    /// This holds the unique categories or the numeric range found in the data.
    pub domain: Option<ScaleDomain>,
}

impl Color {
    /// Creates a new Color encoding for the specified field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            scale: None,  // Will be inferred during data resolution
            domain: None, // Will be populated from data scan
        }
    }

    /// Explicitly sets the scale type for color mapping.
    ///
    /// Use this to override automatic inference, such as forcing a 
    /// numeric field to be treated as categorical data using `Scale::Discrete`.
    ///
    /// # Arguments
    /// * `scale` - The preferred scale type.
    ///
    /// # Returns
    /// Returns `Self` with the updated scale type.
    pub fn with_scale(mut self, scale: Scale) -> Self {
        self.scale = Some(scale);
        self
    }
}

/// Convenience function for creating a color encoding.
///
/// Maps a data field to the color property of marks. This is the 
/// primary entry point for color specifications in the `encode()` method.
///
/// # Arguments
/// * `field` - The name of the data column to use for color encoding.
pub fn color(field: &str) -> Color {
    Color::new(field)
}