use crate::scale::{Expansion, ResolvedScale, Scale, ScaleDomain};

/// Represents an X-axis encoding specification for chart elements.
///
/// This struct follows the "Intent vs. Resolution" pattern:
/// 1. **Intent (Inputs)**: User defines *how* the data should be mapped (field, domain, scale_type).
/// 2. **Resolution (Outputs)**: The engine processes the data and "back-fills" the `resolved_scale`.
///
/// Using `Arc<dyn ScaleTrait>` allows multiple layers in a `LayeredChart` to share the
/// exact same coordinate system instance efficiently without deep-copying data like
/// large color gradient tables.
#[derive(Debug, Clone)]
pub struct X {
    // --- User Configuration (Intent/Inputs) ---
    /// The name of the data column to be mapped to the X-axis.
    pub(crate) field: String,

    /// Display name for the axis, when it differs from `field`.
    ///
    /// Composite marks (violin, contour, ...) rewrite the encoding to point at
    /// generated columns such as `x`/`y`. This label carries the original data
    /// column's name through so the axis stays meaningful. A user-facing
    /// `with_x_label` still takes priority.
    pub(crate) label: Option<String>,

    /// Optional column that supplies the axis *categories* while `field` holds
    /// numeric positions.
    ///
    /// This powers a discrete position axis: the encoded column carries numbers
    /// (`category index + dodge offset`) and this column supplies the integer
    /// ticks and their labels.
    pub(crate) category_field: Option<String>,

    /// The desired scale transformation (e.g., Linear, Log, Discrete).
    /// If `None`, the engine will infer the type from the column's data type.
    pub(crate) scale_type: Option<Scale>,

    /// An explicit data range provided by the user (e.g., [0.0, 100.0]).
    /// This acts as the highest priority override during the training phase.
    pub(crate) domain: Option<ScaleDomain>,

    /// Rules for adding padding/buffer to the ends of the axis domain.
    pub(crate) expansion: Option<Expansion>,

    /// Whether to force the inclusion of zero in the axis range.
    /// This is common for bar charts to avoid misleading visual scales.
    pub(crate) zero: Option<bool>,

    pub(crate) bins: Option<usize>, // bins for continuous encoding value in marks like barchart and histogram

    // --- System Resolution (Result/Outputs) ---
    /// Stores the resolved scale instance. Using RwLock to support
    /// back-filling updates across multiple render calls.
    pub(crate) resolved_scale: ResolvedScale,
}

impl X {
    /// Creates a new X encoding for a specific data field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            label: None,
            category_field: None,
            scale_type: None,
            domain: None,
            expansion: None,
            zero: None,
            bins: None,
            resolved_scale: ResolvedScale::none(),
        }
    }

    /// Overrides the axis title for this channel.
    ///
    /// By default the axis is titled with the data field name; this sets a
    /// display name instead. A `with_x_label` on the finished chart has the
    /// final say.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the preferred scale type (e.g., `Scale::Linear`, `Scale::Log`).
    pub const fn with_scale(mut self, scale_type: Scale) -> Self {
        self.scale_type = Some(scale_type);
        self
    }

    /// Reads the axis categories from `category_field` while keeping the encoded
    /// field as a numeric position.
    ///
    /// The encoded field should hold an integer category index plus an optional
    /// fractional offset (for example a dodged violin's `category + dodge`). The
    /// axis then shows one integer tick per category, labelled from this column.
    pub fn with_category_labels(mut self, category_field: impl Into<String>) -> Self {
        self.category_field = Some(category_field.into());
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
    pub const fn with_expansion(mut self, expansion: Expansion) -> Self {
        self.expansion = Some(expansion);
        self
    }

    /// Determines if the scale must include the zero value.
    pub const fn with_zero(mut self, zero: bool) -> Self {
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
    pub const fn with_bins(mut self, bins: usize) -> Self {
        self.bins = Some(bins);
        self
    }
}

/// Convenience builder function to create a new X encoding.
///
pub fn x(field: &str) -> X {
    X::new(field)
}
