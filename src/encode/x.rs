use crate::scale::Scale;

/// Represents an X-axis encoding for chart elements
///
/// The `X` struct defines how data values should be mapped to the horizontal
/// position of visual elements in a chart. It specifies which data field should
/// be used to determine the x-coordinate of marks and provides additional
/// configuration options for axis scaling and binning.
///
/// X encoding is fundamental to most chart types and can handle both continuous
/// and discrete data. It supports various scale types and binning options for
/// specialized chart types like histograms and bar charts.
#[derive(Debug)]
pub struct X {
    // Default label (polars column name)
    pub(crate) field: String,
    pub(crate) bins: Option<usize>,     // bins for continuous encoding value in marks like barchart and histogram
    pub(crate) scale: Option<Scale>,    // scale type for the axis
    pub(crate) zero: Option<bool>,      // None = auto, Some(true) = force zero, Some(false) = exclude zero
}

impl X {
    fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            bins: None,
            scale: None, // Only applicable for numeric datatypes, so set to None here and will be determined later
            zero: None,  // Default to auto behavior
        }
    }

    /// Sets the number of bins for marks like barchart and histogram
    ///
    /// Configures the number of bins to use when discretizing continuous data
    /// for chart types that require binned data, such as histograms and bar charts.
    /// This is particularly useful for controlling the granularity of data aggregation.
    ///
    /// # Arguments
    /// * `bins` - The number of bins to create from the continuous data
    ///
    /// # Returns
    /// Returns `Self` with the updated bin count
    pub fn with_bins(mut self, bins: usize) -> Self {
        self.bins = Some(bins);
        self
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
    ///
    /// # Returns
    /// Returns `Self` with the updated scale type
    pub fn with_scale(mut self, scale: Scale) -> Self {
        self.scale = Some(scale);
        self
    }

    /// Sets whether to include zero in the axis domain
    ///
    /// Controls the inclusion of zero in the calculated axis range. This can be
    /// important for accurate data representation, especially in bar charts where
    /// excluding zero can exaggerate differences between values.
    ///
    /// # Arguments
    /// * `zero` - A boolean value controlling zero inclusion:
    ///   - `true`: Force include zero in the axis domain
    ///   - `false`: Exclude zero from the axis domain
    ///
    /// # Returns
    /// Returns `Self` with the updated zero inclusion setting
    pub fn with_zero(mut self, zero: bool) -> Self {
        self.zero = Some(zero);
        self
    }
}

/// Top-level convenience function: directly return X
///
/// Provides a convenient way to create an `X` encoding specification that maps
/// a data field to the horizontal position of chart elements.
///
/// # Arguments
/// * `field` - A string slice representing the name of the data column to use for X-axis encoding
///
/// # Returns
/// A new `X` instance configured with the specified field
pub fn x(field: &str) -> X {
    X::new(field)
}
