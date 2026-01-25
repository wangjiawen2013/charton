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
    
    /// The resolved scale instance for text.
    ///
    /// While often unused for simple text labels, having this field maintains 
    /// consistency with other encoding channels and allows for "Identity Scales"
    /// where values are passed through or formatted.
    pub(crate) resolved_scale: Option<Arc<dyn ScaleTrait>>,
}

impl Text {
    /// Creates a new text encoding for a specific data field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            resolved_scale: None,
        }
    }

    /// Injects the resolved scale instance.
    ///
    /// This follows the same back-filling pattern as other encoding channels 
    /// during the resolution phase of a `LayeredChart`.
    pub(crate) fn set_resolved_scale(&mut self, scale: Arc<dyn ScaleTrait>) {
        self.resolved_scale = Some(scale);
    }

    /// Returns the data field name used for text encoding.
    pub fn get_field(&self) -> &str {
        &self.field
    }

    /// Returns a reference to the resolved scale if it has been populated.
    pub fn get_resolved_scale(&self) -> Option<&Arc<dyn ScaleTrait>> {
        self.resolved_scale.as_ref()
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