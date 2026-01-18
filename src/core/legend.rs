use time::OffsetDateTime;

use super::layer::Layer;
use crate::scale::{Scale, ScaleDomain};
use crate::theme::Theme;
use crate::core::utils::estimate_text_width;
use std::collections::HashMap;

/// Represents the physical footprint of a single legend block on the canvas.
/// Used by the LayoutEngine to calculate margins and perform wrapping (bin-packing).
#[derive(Debug, Clone, Copy, Default)]
pub struct LegendSize {
    /// The total horizontal space required for this legend block.
    pub width: f64,
    /// The total vertical space required for this legend block.
    pub height: f64,
}

/// Blueprint for a single legend component.
/// 
/// In the Grammar of Graphics, a legend can unify multiple visual channels 
/// (color, shape, size) if they map to the same underlying data field.
pub struct LegendSpec {
    /// Display title (usually the field name).
    pub title: String,
    /// The data column name used for this mapping.
    pub field: String,
    /// Scale type (Linear, Log, Discrete, Time) determining the visual logic.
    pub scale_type: Scale,
    /// The unique data values or range to be represented.
    pub domain: ScaleDomain,
    
    /// Visual channel flags.
    pub has_color: bool,
    pub has_shape: bool,
    pub has_size: bool,
}

impl LegendSpec {
    /// Estimates the rectangular area needed to render this legend block.
    /// 
    /// This method performs "virtual rendering" to measure:
    /// 1. **Title**: The space for the header text.
    /// 2. **Continuous Colorbar**: A gradient strip if only color is mapped on a continuous scale.
    /// 3. **Discrete/Binned List**: A vertical stack of markers and labels. 
    ///    For 'Size' channels, it simulates markers growing in dimensions.
    pub fn estimate_size(&self, theme: &Theme) -> LegendSize {
        let font_size = theme.legend_font_size.unwrap_or(theme.tick_label_font_size);
        let title_font_size = font_size * 1.1;
        
        // Geometric constants
        let title_to_content_gap = 10.0;
        let marker_to_text_gap = 8.0;
        let item_vertical_gap = 6.0;

        // --- 1. Measure Title ---
        let title_w = estimate_text_width(&self.title, title_font_size);
        let title_h = title_font_size;

        // --- 2. 5-Point Domain Sampling ---
        // We generate representative labels to find the "maximum width" label 
        // across the data range.
        let labels: Vec<String> = match &self.domain {
            ScaleDomain::Categorical(v) => v.clone(),
            ScaleDomain::Continuous(min, max) => {
                (0..5).map(|i| {
                    let val = min + (max - min) * (i as f64 / 4.0);
                    format!("{:.2}", val)
                }).collect()
            },
            ScaleDomain::Temporal(start, end) => {
                let duration = *end - *start;
                (0..5).map(|i| {
                    // Sample 5 points across the time duration
                    let point: OffsetDateTime = *start + duration * (i as i32) / 4;
                    // Format as ISO-like string for measurement
                    point.to_string().split('.').next().unwrap_or("").to_string()
                }).collect()
            }
        };

        // --- 3. Measure Content (Markers + Labels) ---
        let is_discrete = matches!(self.scale_type, Scale::Discrete);
        
        // A Colorbar is only used for continuous color mapping WITHOUT size/shape variations.
        // Size and Shape always require a "list" of distinct symbols to be legible.
        let use_colorbar = !is_discrete && self.has_color && !self.has_size && !self.has_shape;

        let (content_w, content_h) = if use_colorbar {
            // Scenario A: Continuous Colorbar (The Gradient Strip)
            let bar_thickness = 15.0; 
            let bar_length = 120.0; // Fixed aesthetic length for the vertical gradient

            let max_label_w = labels.iter()
                .map(|l| estimate_text_width(l, font_size))
                .fold(0.0, f64::max);

            (bar_thickness + marker_to_text_gap + max_label_w, bar_length)
        } else {
            // Scenario B: Discrete List or Binned Continuous (Size/Shape/Color)
            let max_label_w = labels.iter()
                .map(|l| estimate_text_width(l, font_size))
                .fold(0.0, f64::max);

            let mut total_list_h = 0.0;
            let mut max_marker_w = 12.0;

            for i in 0..labels.len() {
                // Calculate dynamic marker size for this row.
                // If the 'Size' channel is active, markers grow from 5px to 20px.
                let current_marker_size = if self.has_size {
                    if is_discrete { 18.0 } else { 5.0 + (15.0 * (i as f64 / 4.0)) }
                } else {
                    12.0 // Standard swatch/symbol size
                };

                max_marker_w = f64::max(max_marker_w, current_marker_size);
                
                // Each row's height is the maximum of the icon or the text.
                let row_h = f64::max(current_marker_size, font_size);
                total_list_h += row_h;
                
                if i < labels.len() - 1 {
                    total_list_h += item_vertical_gap;
                }
            }

            (max_marker_w + marker_to_text_gap + max_label_w, total_list_h)
        };

        // --- 4. Final Aggregation ---
        LegendSize {
            width: f64::max(title_w, content_w),
            height: title_h + title_to_content_gap + content_h,
        }
    }
}

/// Defines the visual placement of the legend relative to the plot area.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LegendPosition {
    Top,
    Bottom,
    Left,
    Right,
    None,
}

impl Default for LegendPosition {
    fn default() -> Self {
        LegendPosition::Right
    }
}

pub struct LegendManager;

impl LegendManager {
    /// Collects and merges legend requirements from all layers.
    /// Prevents redundant legends for the same field (e.g., if both color and shape map to 'species').
    pub fn collect_legends(layers: &[Box<dyn Layer>]) -> Vec<LegendSpec> {
        let mut specs_map: HashMap<String, LegendSpec> = HashMap::new();

        for layer in layers {
            let encoding = layer.get_encoding();

            if let Some(color_attr) = &encoding.color {
                Self::merge_channel(&mut specs_map, &color_attr.field, "color", layer);
            }
            if let Some(shape_attr) = &encoding.shape {
                Self::merge_channel(&mut specs_map, &shape_attr.field, "shape", layer);
            }
            if let Some(size_attr) = &encoding.size {
                Self::merge_channel(&mut specs_map, &size_attr.field, "size", layer);
            }
        }

        let mut results: Vec<LegendSpec> = specs_map.into_values().collect();
        // Sort for consistent output.
        results.sort_by(|a, b| a.title.cmp(&b.title));
        results
    }

    fn merge_channel(
        map: &mut HashMap<String, LegendSpec>,
        field_name: &str,
        channel: &str,
        layer: &Box<dyn Layer>
    ) {
        let entry = map.entry(field_name.to_string()).or_insert_with(|| {
            LegendSpec {
                title: field_name.to_string(),
                field: field_name.to_string(),
                scale_type: layer.get_scale_type(channel).unwrap_or(Scale::Discrete),
                domain: layer.get_domain(channel).unwrap_or(ScaleDomain::Categorical(vec![])),
                has_color: false,
                has_shape: false,
                has_size: false,
            }
        });

        match channel {
            "color" => entry.has_color = true,
            "shape" => entry.has_shape = true,
            "size" => entry.has_size = true,
            _ => {}
        }
    }
}