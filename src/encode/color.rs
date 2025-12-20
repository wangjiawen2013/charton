/// Represents a color encoding for chart elements
///
/// The `Color` struct defines how data values should be mapped to colors in a visualization.
/// It specifies which data field should be used to determine the color of marks in the chart.
///
/// # Fields
/// * `field` - The name of the data column to use for color encoding
#[derive(Debug)]
pub struct Color {
    pub field: String,
}

impl Color {
    fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
        }
    }
}

/// Top-level convenience function for creating color encodings, consistent with x/y encodings
///
/// This function provides a convenient way to create a `Color` encoding specification
/// that maps a data field to the color of chart elements. It follows the same pattern
/// as the x() and y() encoding functions.
///
/// # Arguments
/// * `field` - A string slice representing the name of the data column to use for color encoding
///
/// # Returns
/// A new `Color` instance configured with the specified field
pub fn color(field: &str) -> Color {
    Color::new(field)
}
