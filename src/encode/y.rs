use crate::scale::{Scale, ScaleDomain, ScaleTrait, Expansion};
use std::sync::Arc;

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
pub struct Y {
    // --- User Configuration (Intent/Inputs) ---
    
    /// The name of the data column to be mapped to the vertical position.
    pub(crate) field: String,

    /// The desired scale transformation (e.g., Linear, Log, Discrete).
    /// If `None`, the engine will infer the type based on the column's data type.
    pub(crate) scale_type: Option<Scale>,

    /// An explicit user-defined data range (e.g., [0.0, 500.0]).
    /// If set, this takes absolute priority over automatic data inference.
    pub(crate) domain: Option<ScaleDomain>,

    /// Rules for adding padding or buffer to the top and bottom of the axis.
    pub(crate) expand: Option<Expansion>,

    /// Whether to force the inclusion of zero in the axis range.
    /// This is crucial for charts like Bar or Area to ensure visual integrity.
    pub(crate) zero: Option<bool>,

    // --- System Resolution (Result/Outputs) ---
    
    /// The concrete, trained scale instance used for rendering.
    ///
    /// This is populated by the `LayeredChart` during the resolution phase. 
    /// We use `Arc` (Atomic Reference Counted) to:
    /// - **Share**: Allow multiple superimposed layers to use the exact same Y-axis instance.
    /// - **Isolate**: Allow faceted charts to hold independent Y-axes by assigning different Arcs.
    /// - **Performance**: Avoid deep-cloning complex scale metadata (like axis labels or color tables).
    pub(crate) resolved_scale: Option<Arc<dyn ScaleTrait>>,
}

impl Y {
    /// Creates a new Y encoding for a specific data field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            scale_type: None,
            domain: None,
            expand: None,
            zero: None,
            resolved_scale: None,
        }
    }

    /// Sets the desired scale type (e.g., `Scale::Linear`, `Scale::Log`).
    pub fn scale(mut self, scale_type: Scale) -> Self {
        self.scale_type = Some(scale_type);
        self
    }

    /// Explicitly sets the data domain (limits) for the Y-axis.
    ///
    /// This prevents the engine from calculating the range from the data.
    pub fn domain(mut self, domain: ScaleDomain) -> Self {
        self.domain = Some(domain);
        self
    }

    /// Configures the expansion padding for the axis.
    pub fn expand(mut self, expand: Expansion) -> Self {
        self.expand = Some(expand);
        self
    }

    /// Determines if the scale must include the zero value.
    pub fn zero(mut self, zero: bool) -> Self {
        self.zero = Some(zero);
        self
    }

    /// Injects the resolved scale instance into the encoding.
    ///
    /// This is called by the `LayeredChart` after combining layer data 
    /// to determine the final coordinate system.
    pub(crate) fn set_resolved_scale(&mut self, scale: Arc<dyn ScaleTrait>) {
        self.resolved_scale = Some(scale);
    }

    /// Returns the name of the data field used for this encoding.
    pub fn field(&self) -> &str {
        &self.field
    }

    /// Returns a reference to the resolved scale if it has been populated.
    /// 
    /// Marks use this to perform the actual mapping from data values to Y-pixels.
    pub fn resolved_scale(&self) -> Option<&Arc<dyn ScaleTrait>> {
        self.resolved_scale.as_ref()
    }
}

/// Convenience builder function to create a new Y encoding.
///
/// # Example
/// ```
/// let encoding = y("sales_volume")
///     .domain(ScaleDomain::Continuous(0.0, 1000.0))
///     .zero(true);
/// ```
pub fn y(field: &str) -> Y {
    Y::new(field)
}