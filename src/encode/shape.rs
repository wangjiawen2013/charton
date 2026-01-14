use crate::scale::{Scale, ScaleDomain};

/// Represents a shape encoding for chart elements.
///
/// The `Shape` struct defines how categorical data values map to geometric
/// shapes (e.g., Circle, Square, Triangle). In a Grammar of Graphics, 
/// shape encoding is strictly discrete.
#[derive(Debug, Clone)]
pub struct Shape {
    /// The name of the data column used for shape mapping.
    pub field: String,
    
    /// The scale type for shapes, which is always Scale::Discrete.
    /// This is hardcoded to ensure logical consistency.
    pub scale: Scale,
    
    /// The resolved data domain (unique categorical values).
    /// This is populated during the data-scan phase before rendering.
    pub domain: Option<ScaleDomain>,
}

impl Shape {
    /// Creates a new Shape encoding for the specified field.
    /// Default scale is set to Discrete, and domain starts as None.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            scale: Scale::Discrete,
            domain: None,
        }
    }
}

/// Convenience function for creating a Shape channel.
///
/// Maps a data field to geometric shapes. Since shapes are used for 
/// categorization, this channel automatically uses a Discrete scale.
///
/// # Arguments
/// * `field` - The name of the data column to use for shape encoding.
pub fn shape(field: &str) -> Shape {
    Shape::new(field)
}