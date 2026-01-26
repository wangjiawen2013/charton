use crate::scale::{ScaleTrait, Scale};
use std::sync::Arc;

/// Represents a complete mapping pipeline from a data field to a visual property.
/// This is the "Source of Truth" for how a specific aesthetic (like color) 
/// behaves across all layers of a chart.
#[derive(Debug, Clone)]
pub struct AestheticMapping {
    /// The name of the data field being mapped (e.g., "mpg", "gear").
    /// This is crucial for merging different aesthetics into a single legend block.
    pub field: String,
    
    /// The high-level type of the scale (Linear, Discrete, etc.).
    /// Used for quick logic branching (e.g., determining if we need a Colorbar).
    pub scale_type: Scale,
    
    /// The mathematical implementation that normalizes data to [0, 1].
    pub scale_impl: Arc<dyn ScaleTrait>,
}

/// GlobalAesthetics maintains the unified visual language of the chart.
/// 
/// Following the Grammar of Graphics, it ensures that if multiple layers 
/// map to the same aesthetic (e.g., Color), they share the same scale and 
/// range, preventing visual inconsistency.
pub struct GlobalAesthetics {
    /// Mapping for the color channel.
    pub color: Option<AestheticMapping>,
    
    /// Mapping for the shape channel (typically discrete).
    pub shape: Option<AestheticMapping>,
    
    /// Mapping for the size channel (typically binned for legends).
    pub size: Option<AestheticMapping>,
}

impl GlobalAesthetics {
    /// Constructs a new `GlobalAesthetics` with fully resolved mappings.
    /// 
    /// This is typically invoked after the "Scale Training" phase, where 
    /// data domains from all layers have been aggregated.
    pub fn new(
        color: Option<AestheticMapping>,
        shape: Option<AestheticMapping>,
        size: Option<AestheticMapping>,
    ) -> Self {
        Self {
            color,
            shape,
            size,
        }
    }

    /// Helper to identify if a specific field is used across multiple aesthetics.
    /// This is the foundation of ggplot2's automatic legend merging.
    pub fn get_mappings_for_field(&self, field_name: &str) -> Vec<(&str, &AestheticMapping)> {
        let mut found = Vec::new();
        if let Some(ref m) = self.color {
            if m.field == field_name { found.push(("color", m)); }
        }
        if let Some(ref m) = self.shape {
            if m.field == field_name { found.push(("shape", m)); }
        }
        if let Some(ref m) = self.size {
            if m.field == field_name { found.push(("size", m)); }
        }
        found
    }
}