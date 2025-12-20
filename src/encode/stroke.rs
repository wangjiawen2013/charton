/// Represents a stroke encoding for chart elements
///
/// The `Stroke` struct defines how data values should be mapped to the stroke
/// color (outline color) of visual elements in a chart. It specifies which data
/// field should be used to determine the color of mark outlines.
///
/// Stroke encoding can be used for both continuous and categorical data and helps
/// visualize additional dimensions by varying the outline color of marks. This is
/// particularly useful when fill color is already used for another encoding.
#[derive(Debug)]
pub struct Stroke {
    pub(crate) field: String,
}

impl Stroke {
    fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
        }
    }
}

/// Convenience function for creating a Stroke channel
///
/// Provides a convenient way to create a `Stroke` encoding specification
/// that maps a data field to the stroke color of chart elements.
///
/// # Arguments
/// * `field` - A string slice representing the name of the data column to use for stroke encoding
///
/// # Returns
/// A new `Stroke` instance configured with the specified field
pub fn stroke(field: &str) -> Stroke {
    Stroke::new(field)
}
