use crate::scale::{Scale, ScaleDomain, ScaleTrait, Expansion};
use std::sync::{Arc, RwLock};

/// Represents an X-axis encoding specification for chart elements.
///
/// This struct follows the "Intent vs. Resolution" pattern:
/// 1. **Intent (Inputs)**: User defines *how* the data should be mapped (field, domain, scale_type).
/// 2. **Resolution (Outputs)**: The engine processes the data and "back-fills" the `resolved_scale`.
///
/// Using `Arc<dyn ScaleTrait>` allows multiple layers in a `LayeredChart` to share the 
/// exact same coordinate system instance efficiently without deep-copying data like 
/// large color gradient tables.
pub struct X {
    // --- User Configuration (Intent/Inputs) ---
    
    /// The name of the data column to be mapped to the X-axis.
    pub(crate) field: String,

    /// The desired scale transformation (e.g., Linear, Log, Discrete).
    /// If `None`, the engine will infer the type from the column's data type.
    pub(crate) scale_type: Option<Scale>,

    /// An explicit data range provided by the user (e.g., [0.0, 100.0]).
    /// This acts as the highest priority override during the training phase.
    pub(crate) domain: Option<ScaleDomain>,

    /// Rules for adding padding/buffer to the ends of the axis domain.
    pub(crate) expand: Option<Expansion>,

    /// Whether to force the inclusion of zero in the axis range.
    /// This is common for bar charts to avoid misleading visual scales.
    pub(crate) zero: Option<bool>,

    pub(crate) bins: Option<usize>, // bins for continuous encoding value in marks like barchart and histogram

    // --- System Resolution (Result/Outputs) ---
    
    /// Stores the resolved scale instance. Using RwLock to support 
    /// back-filling updates across multiple render calls.
    pub(crate) resolved_scale: RwLock<Option<Arc<dyn ScaleTrait>>>,
}

impl X {
    /// Creates a new X encoding for a specific data field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            scale_type: None,
            domain: None,
            expand: None,
            zero: None,
            bins: None,
            resolved_scale: RwLock::new(None),
        }
    }

    /// Sets the preferred scale type (e.g., `Scale::Linear`, `Scale::Log`).
    pub fn with_scale(mut self, scale_type: Scale) -> Self {
        self.scale_type = Some(scale_type);
        self
    }

    /// Explicitly sets the data domain (limits) for this axis.
    ///
    /// Setting this will prevent the engine from automatically calculating 
    /// the range based on the data.
    pub fn with_domain(mut self, domain: ScaleDomain) -> Self {
        self.domain = Some(domain);
        self
    }

    /// Configures the expansion padding for the axis.
    pub fn with_expand(mut self, expand: Expansion) -> Self {
        self.expand = Some(expand);
        self
    }

    /// Determines if the scale must include the zero value.
    pub fn with_zero(mut self, zero: bool) -> Self {
        self.zero = Some(zero);
        self
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
}

/// Convenience builder function to create a new X encoding.
///
/// # Example
/// ```
/// let encoding = x("gdp_per_capita")
///     .with_scale(Scale::Log)
///     .with_zero(false);
/// ```
pub fn x(field: &str) -> X {
    X::new(field)
}