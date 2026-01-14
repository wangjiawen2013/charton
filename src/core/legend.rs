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
        // We use a HashMap keyed by the field name to group different visual 
        // channels that belong to the same data dimension.
        let mut specs_map: HashMap<String, LegendSpec> = HashMap::new();

        for layer in layers {
            // Retrieve the central encoding configuration from the layer
            let encoding = layer.get_encoding();

            // Process Color Encoding: check if the layer has a color channel defined
            if let Some(color_attr) = &encoding.color {
                Self::merge_channel(&mut specs_map, &color_attr.field, "color", layer);
            }
            // Process Shape Encoding: check if the layer has a shape channel defined
            if let Some(shape_attr) = &encoding.shape {
                Self::merge_channel(&mut specs_map, &shape_attr.field, "shape", layer);
            }
            // Process Size Encoding: check if the layer has a size channel defined
            if let Some(size_attr) = &encoding.size {
                Self::merge_channel(&mut specs_map, &size_attr.field, "size", layer);
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
    fn merge_channel(
        map: &mut HashMap<String, LegendSpec>,
        field_name: &str,
        channel: &str,
        layer: &Box<dyn Layer>
    ) {
        // If the field name (e.g., "species") is already in the map, 
        // we retrieve it to add a new visual channel to the existing legend block.
        // Otherwise, we initialize a fresh LegendSpec.
        let entry = map.entry(field_name.to_string()).or_insert_with(|| {
            LegendSpec {
                title: field_name.to_string(),
                field: field_name.to_string(),
                // Extract the final scale and domain info from the layer's 
                // resolved state (calculated by LayeredChart).
                scale_type: layer.get_scale_type(channel).unwrap_or(Scale::Discrete),
                domain: layer.get_domain(channel).unwrap_or(ScaleDomain::Categorical(vec![])),
                has_color: false,
                has_shape: false,
                has_size: false,
            }
        });

        // Set the flag for the specific channel being processed (e.g., "color")
        // This tells the renderer to include color swatches in this legend block.
        match channel {
            "color" => entry.has_color = true,
            "shape" => entry.has_shape = true,
            "size" => entry.has_size = true,
            _ => {}
        }
    }
}