use crate::core::data::AggregateOp;
use crate::scale::{Expansion, ResolvedScale, Scale, ScaleDomain};

/// Stack mode for area/bar charts, following Altair/Vega-Lite convention
#[derive(Clone, Debug, Default, PartialEq)]
pub enum StackMode {
    /// Unstacked/Overlay mode - each area draws from zero baseline independently.
    #[default]
    None,
    /// Stacked mode - each area stacks on top of the previous one (cumulative sum).
    Stacked,
    /// Normalized mode - each vertical slice sums to 1.0 (100% stacked chart).
    Normalize,
    /// Center mode - stacked areas centered around zero baseline (streamgraph).
    Center,
    /// Mirror mode - each series is drawn symmetrically around zero, from
    /// `-value / 2` to `+value / 2`, independently of the other series.
    ///
    /// This is the Vega-Lite `stack: "center"` idea applied per series, and it
    /// is what turns a density curve into a symmetric violin outline when a
    /// facet (or colour) keeps one series per panel.
    Mirror,
}

impl From<&str> for StackMode {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "stacked" => StackMode::Stacked,
            "normalize" => StackMode::Normalize,
            "center" => StackMode::Center,
            "mirror" => StackMode::Mirror,
            _ => StackMode::None,
        }
    }
}

/// Represents a Y-axis encoding specification for chart elements.
///
/// Following the Grammar of Graphics, the `Y` struct separates the
/// declaration of the mapping (how data should be mapped) from the
/// actual execution (the resolved coordinate system).
///
/// ### Lifecycle:
/// 1. **Definition**: Created via `y("field")`. Users specify constraints like `domain` or `zero`.
/// 2. **Resolution**: The `LayeredChart` trains the scale based on the data and constraints.
/// 3. **Back-filling**: A concrete `ScaleTrait` instance is wrapped in an `Arc` and injected into
///    the `resolved_scale` field.
#[derive(Debug, Clone)]
pub struct Y {
    // --- User Configuration (Intent/Inputs) ---
    /// The name of the data column to be mapped to the vertical position.
    pub(crate) field: String,

    /// Display name for the axis, when it differs from `field`.
    ///
    /// See [`X::with_label`](crate::encode::x::X::with_label).
    pub(crate) label: Option<String>,

    /// Optional column that supplies the axis *categories* while `field` holds
    /// numeric positions. See [`X::with_category_labels`](crate::encode::x::X::with_category_labels).
    pub(crate) category_field: Option<String>,

    /// Statistical operation to apply to the data (e.g., Sum, Mean).
    /// Defaults to `AggregateOp::Sum`.
    pub(crate) aggregate: AggregateOp,

    /// The desired scale transformation (e.g., Linear, Log, Discrete).
    /// If `None`, the engine will infer the type based on the column's data type.
    pub(crate) scale_type: Option<Scale>,

    /// An explicit user-defined data range (e.g., [0.0, 500.0]).
    /// If set, this takes absolute priority over automatic data inference.
    pub(crate) domain: Option<ScaleDomain>,

    /// Rules for adding padding or buffer to the top and bottom of the axis.
    pub(crate) expansion: Option<Expansion>,

    /// Whether to force the inclusion of zero in the axis range.
    /// This is crucial for charts like Bar or Area to ensure visual integrity.
    pub(crate) zero: Option<bool>,

    pub(crate) bins: Option<usize>, // bins for continuous encoding value in marks like barchart and histogram

    /// Draw the axis backwards (high values at the bottom).
    pub(crate) reverse: bool,

    // false = raw counts, true = normalize counts to sum to 1 per-group for histogram/bar chart
    pub(crate) normalize: bool,

    // Per X-position (each vertical slice sums to 1.0)
    pub(crate) stack: StackMode,

    // --- System Resolution (Result/Outputs) ---
    /// Stores the resolved scale instance. Using RwLock to support
    /// back-filling updates across multiple render calls.
    pub(crate) resolved_scale: ResolvedScale,
}

impl Y {
    /// Creates a new Y encoding for a specific data field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            label: None,
            category_field: None,
            aggregate: AggregateOp::default(), // Defaults to Sum
            scale_type: None,
            domain: None,
            expansion: None,
            zero: None,
            bins: None,
            reverse: false,
            normalize: false, // Default to false (raw counts)
            stack: StackMode::None,
            resolved_scale: ResolvedScale::none(),
        }
    }

    /// Draws the axis backwards, high values at the bottom.
    ///
    /// A rank axis then shows rank 1 at the top; a depth axis shows the surface
    /// at the top. Expansion and ticks follow the numeric span, not the
    /// direction.
    pub const fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }

    /// Overrides the axis title for this channel.
    ///
    /// By default the axis is titled with the data field name; this sets a
    /// display name instead. A `with_y_label` on the finished chart has the
    /// final say.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the statistical aggregation operation.
    ///
    /// Supports both enum variants and string literals (e.g., "mean", "max").
    ///
    /// ### Example
    /// ```rust,ignore
    /// y("length").with_aggregate("mean") // Using string
    /// y("length").with_aggregate(AggregateOp::Max) // Using enum
    /// ```
    pub fn with_aggregate<A: Into<AggregateOp>>(mut self, op: A) -> Self {
        self.aggregate = op.into();
        self
    }

    /// Sets the desired scale type (e.g., `Scale::Linear`, `Scale::Log`).
    pub const fn with_scale(mut self, scale_type: Scale) -> Self {
        self.scale_type = Some(scale_type);
        self
    }

    /// Reads the axis categories from `category_field` while keeping the encoded
    /// field as a numeric position.
    pub fn with_category_labels(mut self, category_field: impl Into<String>) -> Self {
        self.category_field = Some(category_field.into());
        self
    }

    /// Explicitly sets the data domain (limits) for the Y-axis.
    ///
    /// This prevents the engine from calculating the range from the data.
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
    pub const fn with_normalize(mut self, normalize: bool) -> Self {
        self.normalize = normalize;
        self
    }

    /// Sets the stack mode for area/bar charts.
    ///
    /// Accepts `StackMode` enum or string literals like "stacked", "normalize", "center", "none".
    ///
    /// ### Example
    /// ```rust,ignore
    /// y("value").with_stack(StackMode::Stacked) // Using enum
    /// y("value").with_stack("stacked")          // Using &str
    /// y("value").with_stack("normalize")        // 100% stacked
    /// y("value").with_stack("center")           // Streamgraph
    /// ```
    pub fn with_stack(mut self, stack: impl Into<StackMode>) -> Self {
        self.stack = stack.into();
        self
    }
}

/// Convenience builder function to create a new Y encoding.
///
pub fn y(field: &str) -> Y {
    Y::new(field)
}
