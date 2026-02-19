use crate::scale::{Expansion, ResolvedScale, Scale, ScaleDomain};

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
#[derive(Debug, Clone)]
pub struct Shape {
    // --- User Configuration (Intent/Inputs) ---
    
    /// The name of the data column used for shape mapping.
    pub(crate) field: String,
    
    /// The desired scale transformation. For shapes, this is almost always `Scale::Discrete`.
    pub(crate) scale_type: Option<Scale>,
    
    /// An explicit list of categories or the order of shapes to be used.
    pub(crate) domain: Option<ScaleDomain>,

    /// Rules for adding padding or spacing between the discrete categories.
    pub(crate) expansion: Option<Expansion>,

    // --- System Resolution (Result/Outputs) ---
    
    /// Stores the resolved scale instance. Using RwLock to support 
    /// back-filling updates across multiple render calls.
    pub(crate) resolved_scale: ResolvedScale,
}

impl Shape {
    /// Creates a new Shape encoding for the specified data field.
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            // Shapes default to a Discrete scale logic.
            scale_type: Some(Scale::Discrete), 
            domain: None,
            expansion: None,
            resolved_scale: ResolvedScale::none(),
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
    pub fn with_expansion(mut self, expansion: Expansion) -> Self {
        self.expansion = Some(expansion);
        self
    }
}

/// Convenience builder function to create a new Shape encoding.
///
/// # Example
/// ```
/// // Map the 'category' field to shapes with a default expansion
/// let s = shape("category").with_expansion(Expansion::default());
/// ```
pub fn shape(field: &str) -> Shape {
    Shape::new(field)
}