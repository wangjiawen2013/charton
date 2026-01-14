use crate::scale::{Scale, ScaleDomain};
use crate::error::ChartonError;

/// Represents a size encoding for chart elements.
///
/// The `Size` struct defines how data values should be mapped to the dimensions 
/// of marks (e.g., radius of a bubble, thickness of a line). In standard 
/// visualization theory, size is best suited for continuous numeric data.
#[derive(Debug, Clone)]
pub struct Size {
    /// The name of the data column used for size mapping.
    pub field: String,

    /// The scale type for size mapping (Linear, Log, etc.).
    /// Note: Discrete scales are generally not permitted for size channels.
    pub scale: Scale,

    /// The resolved data domain (e.g., [min, max] range).
    /// This is populated after the data-scan phase to determine the 
    /// boundaries for size mapping.
    pub domain: Option<ScaleDomain>,
}

impl Size {
    /// Creates a new Size encoding for the specified field.
    /// Default scale is set to Log, which is common for wide-ranging numeric data.
    fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            scale: Scale::Log,
            domain: None,
        }
    }

    /// Sets the scale type for the size encoding.
    ///
    /// # Arguments
    /// * `scale` - A `Scale` enum value specifying the transformation (e.g., Linear, Log).
    ///
    /// # Errors
    /// Returns `ChartonError::Scale` if `Scale::Discrete` is provided, as mapping 
    /// categorical labels to an ordered size scale is semantically ambiguous.
    pub fn with_scale(mut self, scale: Scale) -> Result<Self, ChartonError> {
        if matches!(scale, Scale::Discrete) {
            return Err(ChartonError::Scale(
                "Size encoding cannot use Scale::Discrete as size requires continuous data".to_string()
            ));
        }
        
        self.scale = scale;
        Ok(self)
    }
}

/// Convenience function for creating a Size channel.
///
/// Maps a data field to the size of chart elements. 
///
/// # Arguments
/// * `field` - The name of the data column to use for size encoding.
pub fn size(field: &str) -> Size {
    Size::new(field)
}