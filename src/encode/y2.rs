/// Represents a secondary Y-axis encoding for chart elements
///
/// The `Y2` struct defines how data values should be mapped to a secondary vertical
/// position in charts that support dual Y-axes. It specifies which data field should
/// be used to determine the y2-coordinate of marks, typically used for range-based
/// visualizations like error bars, candlestick charts, or rule marks that span
/// between two Y positions.
///
/// Y2 encoding is particularly useful for marks like rules where you need to specify
/// both a starting and ending Y position, or for overlaying multiple datasets with
/// different scales on the same chart.
#[derive(Debug)]
pub struct Y2 {
    // polars column name
    pub(crate) field: String,
}

impl Y2 {
    fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
        }
    }
}

/// Top-level convenience function: directly return Y2
///
/// Provides a convenient way to create a `Y2` encoding specification that maps
/// a data field to the secondary vertical position of chart elements.
///
/// # Arguments
/// * `field` - A string slice representing the name of the data column to use for secondary Y-axis encoding
///
/// # Returns
/// A new `Y2` instance configured with the specified field
pub fn y2(field: &str) -> Y2 {
    Y2::new(field)
}
