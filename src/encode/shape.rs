use crate::scale::{Scale, ScaleDomain, ScaleTrait, Expansion};
use std::sync::Arc;

/// Represents a shape encoding specification for chart elements.
///
/// The `Shape` struct defines how categorical data values map to geometric
/// symbols (e.g., Circle, Square, Triangle). 
///
/// ### Architecture Note:
/// Following the "Intent vs. Resolution" pattern, this struct holds the configuration 
/// until the engine scans the data and assigns specific symbols to each unique 
/// category. Using `Arc<dyn ScaleTrait>` ensures that even if different marks 
/// (points, ticks, or custom symbols) are used in superimposed layers, they 
/// consistently map the same data category to the same visual shape.
pub struct Shape {
    // --- User Configuration (Intent/Inputs) ---
    
    /// The name of the data column used for shape mapping.
    pub(crate) field: String,
    
    /// The desired scale transformation. For shapes, this is almost always `Scale::Discrete`.
    pub(crate) scale_type: Option<Scale>,
    
    /// An explicit list of categories or the order of shapes to be used.
    pub(crate) domain: Option<ScaleDomain>,

    /// Rules for adding padding or spacing between the discrete categories.
    pub(crate) expand: Option<Expansion>,

    // --- System Resolution (Result/Outputs) ---
    
    /// The concrete, trained shape scale instance, shared via Arc.
    ///
    /// Once populated by the `LayeredChart`, this instance contains the mapping 
    /// table between data strings and geometric identifiers. Using `Arc` prevents 
    /// redundant palette logic across multiple chart layers.
    pub(crate) resolved_scale: Option<Arc<dyn ScaleTrait>>,
}

impl Shape {
    /// Creates a new Shape encoding for the specified data field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            // Shapes default to a Discrete scale logic.
            scale_type: Some(Scale::Discrete), 
            domain: None,
            expand: None,
            resolved_scale: None,
        }
    }

    /// Sets the desired scale type. Usually kept as Discrete for shapes.
    pub fn with_scale(mut self, scale_type: Scale) -> Self {
        self.scale_type = Some(scale_type);
        self
    }

    /// Explicitly sets the categorical domain for the shape scale.
    /// 
    /// This is used to define which categories get mapped and in what order.
    pub fn with_domain(mut self, domain: ScaleDomain) -> Self {
        self.domain = Some(domain);
        self
    }

    /// Configures the expansion padding (spacing) for the categorical axis.
    pub fn with_expand(mut self, expand: Expansion) -> Self {
        self.expand = Some(expand);
        self
    }

    /// Injects the resolved shape scale instance.
    ///
    /// Back-filled by the engine after it has identified the unique data 
    /// categories and assigned them to the symbol palette.
    pub(crate) fn set_resolved_scale(&mut self, scale: Arc<dyn ScaleTrait>) {
        self.resolved_scale = Some(scale);
    }

    /// Returns the data field name used for shape encoding.
    pub fn get_field(&self) -> &str {
        &self.field
    }

    /// Returns a reference to the resolved scale instance.
    /// 
    /// Marks call this to retrieve the specific symbol ID for a data point.
    pub fn get_resolved_scale(&self) -> Option<&Arc<dyn ScaleTrait>> {
        self.resolved_scale.as_ref()
    }
}

/// Convenience builder function to create a new Shape encoding.
///
/// # Example
/// ```
/// // Map the 'category' field to shapes with a default expansion
/// let s = shape("category").with_expand(Expansion::default());
/// ```
pub fn shape(field: &str) -> Shape {
    Shape::new(field)
}