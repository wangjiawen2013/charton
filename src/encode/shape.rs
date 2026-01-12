use crate::scale::Scale;

/// Represents a shape encoding for chart elements
///
/// The `Shape` struct defines how data values should be mapped to the geometric
/// shape of visual elements in a chart. It specifies which data field should be
/// used to determine the shape of marks.
///
/// Shape encoding is typically used for categorical (discrete) data and can help
/// differentiate between different categories or groups in the visualization.
/// Common shapes include circles, squares, triangles, and other geometric forms.
#[derive(Debug)]
pub struct Shape {
    pub(crate) field: String,
    pub(crate) scale: Option<Scale>,    // scale type for shape
}

impl Shape {
    fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            scale: None, // Will be initialized when encoding
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
    ///   - `Discrete`: For categorical data with distinct categories
    ///   - `Temporal`: For time data
    ///
    /// # Returns
    /// Returns `Self` with the updated scale type
    pub fn with_scale(mut self, scale: Scale) -> Self {
        self.scale = Some(scale);
        self
    }
}

/// Convenience function for creating a Shape channel
///
/// Provides a convenient way to create a `Shape` encoding specification
/// that maps a data field to the geometric shape of chart elements.
///
/// # Arguments
/// * `field` - A string slice representing the name of the data column to use for shape encoding
///
/// # Returns
/// A new `Shape` instance configured with the specified field
pub fn shape(field: &str) -> Shape {
    Shape::new(field)
}
