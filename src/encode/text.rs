use crate::scale::ScaleTrait;
use std::sync::Arc;

/// Represents a text encoding specification.
///
/// The `Text` struct defines how data values should be mapped to textual content
/// in a chart. It is primarily used by `Text` marks to display labels, 
/// annotations, or categorical information directly on the visualization.
///
/// ### Architecture Note:
/// Unlike positional or visual encodings, `Text` is often a direct mapping 
/// of data values to strings. However, it still holds a `resolved_scale` to 
/// allow for potential future features like data formatting (e.g., date 
/// formatting or number rounding) before the text is rendered.
pub struct Text {
    // --- User Configuration (Intent/Inputs) ---
    
    /// The name of the data column to be used for text content.
    pub(crate) field: String,

    // --- System Resolution (Result/Outputs) ---
    
    /// Stores the concrete, trained scale instance for rendering.
    /// We use `OnceLock` to provide interior mutability, allowing the global 
    /// resolution phase to "back-fill" this field while the layer is held by an `Arc`.
    pub(crate) resolved_scale: std::sync::OnceLock<Arc<dyn ScaleTrait>>,
}

impl Text {
    /// Creates a new text encoding for a specific data field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            resolved_scale: std::sync::OnceLock::new(),
        }
    }
}

/// Convenience builder function to create a new Text encoding.
///
/// # Example
/// ```
/// // Map the 'city_name' column to labels on a map or chart
/// let t = text("city_name");
/// ```
pub fn text(field: &str) -> Text {
    Text::new(field)
}