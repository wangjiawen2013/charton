/// Represents a theta encoding for chart elements
///
/// The `Theta` struct defines how data values should be mapped to the angular
/// position or angle of visual elements in a chart. It specifies which data
/// field should be used to determine the angular component of marks.
///
/// Theta encoding is typically used for polar coordinates and radial visualizations
/// such as pie charts, donut charts, and radial bar charts. It represents the
/// angular dimension of data points in these chart types.
#[derive( Debug, Clone)]
pub struct Theta {
    pub(crate) field: String,
}

impl Theta {
    fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
        }
    }
}

/// Convenience function for creating a Theta channel
///
/// Provides a convenient way to create a `Theta` encoding specification
/// that maps a data field to the angular position of chart elements.
///
/// # Arguments
/// * `field` - A string slice representing the name of the data column to use for theta encoding
///
/// # Returns
/// A new `Theta` instance configured with the specified field
pub fn theta(field: &str) -> Theta {
    Theta::new(field)
}
