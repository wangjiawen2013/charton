/// Represents a stroke width encoding for chart elements
///
/// The `StrokeWidth` struct defines how data values should be mapped to the stroke
/// width (outline thickness) of visual elements in a chart. It specifies which data
/// field should be used to determine the thickness of mark outlines.
///
/// Stroke width encoding is typically used for continuous data and can help visualize
/// additional dimensions such as importance, confidence, or magnitude of data points.
/// Thicker strokes usually represent higher values, while thinner strokes represent lower values.
#[derive(Debug)]
pub struct StrokeWidth {
    pub(crate) field: String,
}

impl StrokeWidth {
    fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
        }
    }
}

/// Convenience function for creating a StrokeWidth channel
///
/// Provides a convenient way to create a `StrokeWidth` encoding specification
/// that maps a data field to the stroke width of chart elements.
///
/// # Arguments
/// * `field` - A string slice representing the name of the data column to use for stroke width encoding
///
/// # Returns
/// A new `StrokeWidth` instance configured with the specified field
pub fn stroke_width(field: &str) -> StrokeWidth {
    StrokeWidth::new(field)
}
