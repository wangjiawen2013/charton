use crate::scale::{Scale, ScaleDomain, ScaleTrait, Expansion};
use crate::error::ChartonError;
use std::sync::Arc;

/// Represents a size encoding specification for chart elements.
///
/// The `Size` struct defines how data values should be mapped to the dimensions 
/// of marks, such as the radius of a bubble in a scatter plot or the thickness 
/// of a line. 
///
/// ### Architecture Note:
/// Size follows the "Intent vs. Resolution" pattern. During the training phase,
/// the engine ensures that the data range is mapped to a sensible range of 
/// visual sizes (e.g., mapping a domain of [0, 1000] to a range of [1px, 20px]).
pub struct Size {
    // --- User Configuration (Intent/Inputs) ---
    
    /// The name of the data column used for size mapping.
    pub(crate) field: String,

    /// The scale type for size mapping (e.g., Linear, Log).
    /// Defaults to `Scale::Linear`. Note: `Scale::Discrete` is typically disallowed.
    pub(crate) scale_type: Option<Scale>,

    /// An explicit user-defined data range for size mapping.
    pub(crate) domain: Option<ScaleDomain>,

    /// Rules for adding padding or buffer to the ends of the size domain.
    pub(crate) expand: Option<Expansion>,

    // --- System Resolution (Result/Outputs) ---
    
    /// The concrete, trained size scale instance, shared via Arc.
    ///
    /// This is populated by the `LayeredChart` resolution phase. Using `Arc`
    /// ensures that all marks in a chart (and the size legend) interpret 
    /// the data values using the exact same scaling factor.
    pub(crate) resolved_scale: Option<Arc<dyn ScaleTrait>>,
}

impl Size {
    /// Creates a new Size encoding for the specified data field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            scale_type: Some(Scale::Linear),
            domain: None,
            expand: None,
            resolved_scale: None,
        }
    }

    /// Sets the scale type for the size encoding (e.g., Linear, Log, Sqrt).
    ///
    /// # Errors
    /// Returns `ChartonError::Scale` if `Scale::Discrete` is provided, as size 
    /// is semantically intended for continuous or ordered data.
    pub fn with_scale(mut self, scale_type: Scale) -> Result<Self, ChartonError> {
        if matches!(scale_type, Scale::Discrete) {
            return Err(ChartonError::Scale(
                "Size encoding cannot use Scale::Discrete as size requires continuous data".to_string()
            ));
        }
        self.scale_type = Some(scale_type);
        Ok(self)
    }

    /// Explicitly sets the data domain for the size scale.
    pub fn with_domain(mut self, domain: ScaleDomain) -> Self {
        self.domain = Some(domain);
        self
    }

    /// Configures the expansion padding for the size scale.
    pub fn with_expand(mut self, expand: Expansion) -> Self {
        self.expand = Some(expand);
        self
    }

    /// Injects the resolved size scale instance.
    pub(crate) fn set_resolved_scale(&mut self, scale: Arc<dyn ScaleTrait>) {
        self.resolved_scale = Some(scale);
    }

    /// Returns the name of the data field used for size encoding.
    pub fn get_field(&self) -> &str {
        &self.field
    }

    /// Returns a reference to the resolved scale instance.
    pub fn get_resolved_scale(&self) -> Option<&Arc<dyn ScaleTrait>> {
        self.resolved_scale.as_ref()
    }
}

/// Convenience builder function to create a new Size encoding.
///
/// # Example
/// ```
/// let s = size("population").with_domain(ScaleDomain::Continuous(0.0, 1e9));
/// ```
pub fn size(field: &str) -> Size {
    Size::new(field)
}