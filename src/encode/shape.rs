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
}

impl Shape {
    fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
        }
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