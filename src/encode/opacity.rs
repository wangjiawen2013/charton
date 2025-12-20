/// Represents an opacity encoding for chart elements
///
/// The `Opacity` struct defines how data values should be mapped to the opacity
/// (transparency) of visual elements in a chart. It specifies which data field
/// should be used to determine the transparency level of marks.
///
/// Opacity encoding is typically used for continuous data and can help visualize
/// additional dimensions in the data, such as confidence levels, density, or importance.
#[derive(Debug)]
pub struct Opacity {
    pub(crate) field: String,
}

impl Opacity {
    fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
        }
    }
}

/// Convenience function for creating an Opacity channel
///
/// Provides a convenient way to create an `Opacity` encoding specification
/// that maps a data field to the opacity of chart elements.
///
/// # Arguments
/// * `field` - A string slice representing the name of the data column to use for opacity encoding
///
/// # Returns
/// A new `Opacity` instance configured with the specified field
pub fn opacity(field: &str) -> Opacity {
    Opacity::new(field)
}
