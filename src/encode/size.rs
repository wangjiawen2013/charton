use crate::scale::Scale;
use crate::error::ChartonError;

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
    pub(crate) scale: Scale,    // scale type for size
}

impl Size {
    fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            scale: Scale::Log,
        }
    }

    /// Sets the scale type for the axis
    ///
    /// Configures the scaling function used to map data values to positional coordinates.
    /// Different scale types are appropriate for different data distributions and
    /// visualization needs.
    ///
    /// # Arguments
    /// * `scale` - A `Scale` enum value specifying the axis scale type
    ///   - `Linear`: Standard linear scale for uniformly distributed data
    ///   - `Log`: Logarithmic scale for exponentially distributed data
    ///   - `Discrete`: For categorical data with distinct categories (not allowed for size)
    ///   - `Temporal`: For time data
    ///
    /// # Returns
    /// Returns `Result<Self, ChartonError>` with the updated size encoding or an error
    /// 
    /// # Errors
    /// Returns `ChartonError::Scale` if `Scale::Discrete` is provided, as size encoding
    /// requires continuous data and cannot use discrete scales.
    pub fn with_scale(mut self, scale: Scale) -> Result<Self, ChartonError> {
        if matches!(scale, Scale::Discrete) {
            return Err(ChartonError::Scale(
                "Size encoding cannot use Scale::Discrete as size requires continuous data".to_string()
            ));
        }
        
        self.scale = scale;
        Ok(self)
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
