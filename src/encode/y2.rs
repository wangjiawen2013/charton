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
    
    /// The resolved scale instance shared with the primary Y-axis.
    ///
    /// This is populated by the `LayeredChart` resolution phase. It should 
    /// point to the exact same `Arc` instance as the primary `Y` encoding 
    /// to ensure they share the same domain and pixel mapping.
    pub(crate) resolved_scale: Option<Arc<dyn ScaleTrait>>,
}

impl Y2 {
    /// Creates a new Y2 encoding for the specified data field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            resolved_scale: None,
        }
    }

    /// Injects the resolved scale instance.
    /// 
    /// In most cases, the engine will pass the same `Arc` used by the `Y` encoding.
    pub(crate) fn set_resolved_scale(&mut self, scale: Arc<dyn ScaleTrait>) {
        self.resolved_scale = Some(scale);
    }

    /// Returns a reference to the resolved scale.
    /// 
    /// Marks like Area or Bar use this to calculate the "secondary" pixel coordinate.
    pub fn get_resolved_scale(&self) -> Option<&Arc<dyn ScaleTrait>> {
        self.resolved_scale.as_ref()
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