use crate::coord::Scale;

/// Represents a Y-axis encoding for chart elements
///
/// The `Y` struct defines how data values should be mapped to the vertical
/// position of visual elements in a chart. It specifies which data field should
/// be used to determine the y-coordinate of marks and provides additional
/// configuration options for axis scaling, binning, normalization, and stacking.
///
/// Y encoding is fundamental to most chart types and can handle both continuous
/// and discrete data. It supports various scale types and specialized options for
/// bar charts and histograms, including normalization and stacking capabilities.
#[derive(Debug)]
pub struct Y {
    // Default label (polars column name)
    pub(crate) field: String,
    pub(crate) bins: Option<usize>, // bins for continuous encoding value in marks like barchart and histogram
    pub(crate) scale: Option<Scale>, // scale type for the axis
    pub(crate) zero: Option<bool>, // None = auto, Some(true) = force zero, Some(false) = exclude zero
    pub(crate) normalize: bool, // false = raw counts, true = normalize counts to sum to 1 for histogram/bar chart
    pub(crate) stack: bool,     // false = regular bar chart, true = stacked bar chart
}

impl Y {
    fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            bins: None,
            scale: None, // The actual scale type will be determined later using determine_scale_for_dtype
            zero: None,  // Default to auto behavior
            normalize: false, // Default to false (raw counts)
            stack: false, // Default to false (regular bar chart)
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
    ///   - `false`: Don't force zero from the axis domain, leave it as it is
    ///
    /// # Returns
    /// Returns `Self` with the updated zero inclusion setting
    pub fn with_zero(mut self, zero: bool) -> Self {
        self.zero = Some(zero);
        self
    }

    /// Sets whether to normalize histogram counts or bar chart values
    ///
    /// Controls whether the y-axis values should represent raw counts or normalized
    /// proportions. Normalized values sum to 1, making it easier to compare distributions
    /// across different datasets or categories.
    ///
    /// # Arguments
    /// * `normalize` - A boolean value controlling normalization:
    ///   - `true`: Normalize counts so they sum to 1 (proportions)
    ///   - `false`: Use raw counts (default)
    ///
    /// # Returns
    /// Returns `Self` with the updated normalization setting
    pub fn with_normalize(mut self, normalize: bool) -> Self {
        self.normalize = normalize;
        self
    }

    /// Sets whether to stack bars
    ///
    /// Controls whether bars in bar charts should be displayed as separate entities
    /// or stacked on top of each other. Stacked bars are useful for showing part-to-whole
    /// relationships within categories.
    ///
    /// # Arguments
    /// * `stack` - A boolean value controlling bar stacking:
    ///   - `true`: Stack bars to show cumulative values
    ///   - `false`: Display bars separately (default)
    ///
    /// # Returns
    /// Returns `Self` with the updated stacking setting
    pub fn with_stack(mut self, stack: bool) -> Self {
        self.stack = stack;
        self
    }
}

/// Top-level convenience function: directly return Y
///
/// Provides a convenient way to create a `Y` encoding specification that maps
/// a data field to the vertical position of chart elements.
///
/// # Arguments
/// * `field` - A string slice representing the name of the data column to use for Y-axis encoding
///
/// # Returns
/// A new `Y` instance configured with the specified field
pub fn y(field: &str) -> Y {
    Y::new(field)
}
