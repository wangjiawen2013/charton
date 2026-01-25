use crate::scale::{Scale, ScaleDomain, ScaleTrait, Expansion};
use std::sync::Arc;

/// Represents a theta (angular) encoding specification.
///
/// The `Theta` struct defines how data values should be mapped to the angular
/// position or arc length of visual elements, primarily in polar coordinates.
/// It is the core encoding for pie charts, donut charts, and radial plots.
///
/// ### Architecture Note:
/// Similar to `X` and `Y`, `Theta` requires a resolution phase where the data 
/// domain is mapped to an angular range (e.g., [0, 2π]). Using `Arc<dyn ScaleTrait>`
/// ensures that different sectors in a radial chart remain perfectly aligned.
pub struct Theta {
    // --- User Configuration (Intent/Inputs) ---
    
    /// The name of the data column used for the angular component.
    pub(crate) field: String,

    /// The desired scale transformation (e.g., Linear, Sqrt).
    /// If `None`, the engine usually defaults to a Linear scale for Theta.
    pub(crate) scale_type: Option<Scale>,

    /// An explicit user-defined data range for angular mapping.
    pub(crate) domain: Option<ScaleDomain>,

    /// Rules for adding padding or buffer to the angular domain.
    pub(crate) expand: Option<Expansion>,

    /// Whether to force the inclusion of zero in the angular range.
    /// Often used in radial charts to ensure slices represent absolute proportions.
    pub(crate) zero: Option<bool>,

    // --- System Resolution (Result/Outputs) ---
    
    /// The concrete, trained angular scale instance.
    ///
    /// This is populated by the `LayeredChart` resolution phase. The `Arc`
    /// allows shared ownership between the chart rendering and potentially 
    /// radial axis or legend generators.
    pub(crate) resolved_scale: Option<Arc<dyn ScaleTrait>>,
}

impl Theta {
    /// Creates a new Theta encoding for a specific data field.
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

    /// Sets the desired scale type (e.g., Linear).
    pub fn with_scale(mut self, scale_type: Scale) -> Self {
        self.scale_type = Some(scale_type);
        self
    }

    /// Explicitly sets the data domain for the angular scale.
    pub fn with_domain(mut self, domain: ScaleDomain) -> Self {
        self.domain = Some(domain);
        self
    }

    /// Configures the expansion padding for the angular axis.
    pub fn with_expand(mut self, expand: Expansion) -> Self {
        self.expand = Some(expand);
        self
    }

    /// Determines if the scale must include the zero value.
    pub fn with_zero(mut self, zero: bool) -> Self {
        self.zero = Some(zero);
        self
    }

    /// Injects the resolved angular scale instance.
    pub(crate) fn set_resolved_scale(&mut self, scale: Arc<dyn ScaleTrait>) {
        self.resolved_scale = Some(scale);
    }

    /// Returns the name of the data field used for this encoding.
    pub fn get_field(&self) -> &str {
        &self.field
    }

    /// Returns a reference to the resolved scale instance.
    pub fn get_resolved_scale(&self) -> Option<&Arc<dyn ScaleTrait>> {
        self.resolved_scale.as_ref()
    }
}

/// Convenience builder function to create a new Theta encoding.
///
/// # Example
/// ```
/// // Map the 'revenue' column to slices in a pie chart
/// let t = theta("revenue").zero(true);
/// ```
pub fn theta(field: &str) -> Theta {
    Theta::new(field)
}