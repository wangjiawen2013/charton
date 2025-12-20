/// Text encoding specification
///
/// The `Text` struct defines how data values should be mapped to text content
/// in a chart. It specifies which data field should be used to determine the
/// text labels or annotations displayed at specific data points.
///
/// Text encoding is primarily used with text marks to display data-driven labels,
/// annotations, or categorical information directly on the chart. This can help
/// identify specific data points or provide additional context.
#[derive(Debug, Clone)]
pub struct Text {
    /// Data field name
    pub(crate) field: String,
}

impl Text {
    /// Create a new text encoding for a field
    fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
        }
    }
}

/// Create a text encoding for a field
///
/// Provides a convenient way to create a `Text` encoding specification
/// that maps a data field to text content in the chart.
///
/// # Arguments
/// * `field` - A string slice representing the name of the data column to use for text encoding
///
/// # Returns
/// A new `Text` instance configured with the specified field
pub fn text(field: &str) -> Text {
    Text::new(field)
}
