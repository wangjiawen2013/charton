/// Represents a size encoding for chart elements
///
/// The `Size` struct defines how data values should be mapped to the size
/// of visual elements in a chart. It specifies which data field should be
/// used to determine the dimensions of marks.
///
/// Size encoding is typically used for continuous data and can help visualize
/// the magnitude or importance of data points. Larger sizes usually represent
/// higher values, while smaller sizes represent lower values.
#[derive(Debug)]
pub struct Size {
    pub(crate) field: String,
}

impl Size {
    fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
        }
    }
}

/// Convenience function for creating a Size channel
///
/// Provides a convenient way to create a `Size` encoding specification
/// that maps a data field to the size of chart elements.
///
/// # Arguments
/// * `field` - A string slice representing the name of the data column to use for size encoding
///
/// # Returns
/// A new `Size` instance configured with the specified field
pub fn size(field: &str) -> Size {
    Size::new(field)
}
