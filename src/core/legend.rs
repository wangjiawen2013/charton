use super::layer::Layer;
use crate::scale::{Scale, ScaleDomain};
use std::collections::HashMap;

/// LegendSpec represents the blueprint for a single legend block.
/// In Grammar of Graphics, a legend can represent multiple visual channels 
/// (e.g., color and shape) if they map to the same data field.
pub struct LegendSpec {
    /// The display title of the legend (usually the field name)
    pub title: String,
    /// The underlying data column name used for this mapping
    pub field: String,
    /// The scale type (e.g., Linear, Log, Discrete) to determine how labels are formatted
    pub scale_type: crate::scale::Scale,
    /// The unique data values (domain) that need to be represented in the legend
    pub domain: crate::scale::ScaleDomain,
    
    // Flags to indicate which visual properties are merged into this legend block
    /// If true, the legend will display color swatches
    pub has_color: bool,
    /// If true, the legend will display geometric shape symbols
    pub has_shape: bool,
    /// If true, the legend will display symbols of varying sizes
    pub has_size: bool,
}

pub struct LegendManager;

impl LegendManager {
    /// Iterates through all chart layers to collect and unify legend requirements.
    /// This is the "brain" that prevents duplicate legends for the same data field.
    pub fn collect_legends(layers: &[Box<dyn Layer>]) -> Vec<LegendSpec> {
        use std::collections::HashMap;
        
        // We use a HashMap keyed by the field name to group different visual 
        // channels that belong to the same data dimension.
        let mut specs_map: HashMap<String, LegendSpec> = HashMap::new();

        for layer in layers {
            // Retrieve all visual encodings defined in this specific layer
            let encodings = layer.get_all_encodings(); 

            // Process Color Encoding
            if let Some(color_enc) = &encodings.color {
                Self::merge_encoding(&mut specs_map, color_enc, "color", layer);
            }
            // Process Shape Encoding
            if let Some(shape_enc) = &encodings.shape {
                Self::merge_encoding(&mut specs_map, shape_enc, "shape", layer);
            }
            // Process Size Encoding
            if let Some(size_enc) = &encodings.size {
                Self::merge_encoding(&mut specs_map, size_enc, "size", layer);
            }
        }

        // Convert the map to a vector and sort it alphabetically by title 
        // to ensure deterministic rendering order across different runs.
        let mut results: Vec<LegendSpec> = specs_map.into_values().collect();
        results.sort_by(|a, b| a.title.cmp(&b.title));
        results
    }

    /// Helper function to either create a new LegendSpec or update an existing 
    /// one with a new visual channel (e.g., adding 'shape' to a 'color' legend).
    fn merge_encoding(
        map: &mut HashMap<String, LegendSpec>,
        enc: &crate::encode::encoding::Encoding, 
        channel: &str,
        layer: &Box<dyn Layer>
    ) {
        let entry = map.entry(enc.field.clone()).or_insert_with(|| {
            // If the field isn't in the map yet, initialize a fresh Spec
            LegendSpec {
                title: enc.field.clone(),
                field: enc.field.clone(),
                // Extract scale/domain info from the layer's internal logic
                scale_type: layer.get_scale_type(channel).unwrap_or(Scale::Discrete),
                domain: layer.get_domain(channel).unwrap_or(ScaleDomain::Categorical(vec![])),
                has_color: false,
                has_shape: false,
                has_size: false,
            }
        });

        // Set the flag for the specific channel being processed
        match channel {
            "color" => entry.has_color = true,
            "shape" => entry.has_shape = true,
            "size" => entry.has_size = true,
            _ => {}
        }
    }
}