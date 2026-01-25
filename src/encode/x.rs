use crate::scale::{Scale, ScaleDomain, ScaleTrait, Expansion};
use std::sync::Arc;

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

    // --- System Resolution (Result/Outputs) ---
    
    /// The concrete, trained scale instance used for actual rendering.
    ///
    /// This field is initially `None` and is populated by the `LayeredChart` 
    /// during the resolution phase. Once set, it contains the final 
    /// mathematical mapping logic (e.g., domain to pixels).
    /// 
    /// We use `Arc` (Atomic Reference Counted) to:
    /// - Share a single scale instance across multiple superimposed layers.
    /// - Avoid expensive deep clones of complex scales (like those with custom color gradients).
    /// - Ensure thread-safety if rendering is parallelized in the future.
    pub(crate) resolved_scale: Option<Arc<dyn ScaleTrait>>,
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
            resolved_scale: None,
        }
    }

    /// Sets the preferred scale type (e.g., `Scale::Linear`, `Scale::Log`).
    pub fn scale(mut self, scale_type: Scale) -> Self {
        self.scale_type = Some(scale_type);
        self
    }

    /// Explicitly sets the data domain (limits) for this axis.
    ///
    /// Setting this will prevent the engine from automatically calculating 
    /// the range based on the data.
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

    /// Back-fills the resolved scale instance into the encoding.
    ///
    /// This is called by the rendering engine after it has combined data 
    /// and configurations to create the final coordinate system.
    pub(crate) fn set_resolved_scale(&mut self, scale: Arc<dyn ScaleTrait>) {
        self.resolved_scale = Some(scale);
    }

    /// Returns the name of the data field used for this encoding.
    pub fn field(&self) -> &str {
        &self.field
    }

    /// Returns a reference to the resolved scale if it has been populated.
    /// 
    /// Marks should call this during their `render` pass to convert 
    /// data values into visual coordinates.
    pub fn resolved_scale(&self) -> Option<&Arc<dyn ScaleTrait>> {
        self.resolved_scale.as_ref()
    }
}

/// Convenience builder function to create a new X encoding.
///
/// # Example
/// ```
/// let encoding = x("gdp_per_capita")
///     .scale(Scale::Log)
///     .zero(false);
/// ```
pub fn x(field: &str) -> X {
    X::new(field)
}