use crate::scale::ScaleTrait;
use std::sync::Arc;

/// Represents a secondary Y-axis encoding specification (Y2).
///
/// `Y2` is typically used for visual marks that require two vertical coordinates,
/// such as the baseline of an Area chart, the second endpoint of a Rule, 
/// or the "start" value of a Bar.
///
/// ### Architecture Note:
/// Unlike `X` or `Y`, `Y2` does not usually define its own scale logic (domain, type, etc.).
/// Instead, it maps a different data field onto the **same** scale as `Y`. 
/// For instance, in an Area chart, `Y` might map to "high_price" and `Y2` to "low_price",
/// but both must use the same vertical coordinate system to be visually coherent.
pub struct Y2 {
    // --- User Configuration (Intent/Inputs) ---
    
    /// The name of the data column to be mapped to the secondary vertical position.
    pub(crate) field: String,

    // --- System Resolution (Result/Outputs) ---
    
    /// Stores the concrete, trained scale instance for rendering.
    /// We use `OnceLock` to provide interior mutability, allowing the global 
    /// resolution phase to "back-fill" this field while the layer is held by an `Arc`.
    pub(crate) resolved_scale: std::sync::OnceLock<Arc<dyn ScaleTrait>>,
}

impl Y2 {
    /// Creates a new Y2 encoding for the specified data field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            resolved_scale: std::sync::OnceLock::new(),
        }
    }
}

/// Convenience builder function to create a new Y2 encoding.
///
/// # Example
/// ```
/// // Mapping an area between 'min_temp' and 'max_temp'
/// let area_encoding = encoding()
///     .y("max_temp")
///     .y2("min_temp");
/// ```
pub fn y2(field: &str) -> Y2 {
    Y2::new(field)
}