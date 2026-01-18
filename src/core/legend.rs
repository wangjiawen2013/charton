use super::layer::Layer;
use crate::scale::{Scale, ScaleDomain};
use crate::theme::Theme;
use crate::core::utils::estimate_text_width;
use std::collections::HashMap;

/// Represents the physical footprint of a legend component.
/// A single block can span multiple columns if it contains many items.
#[derive(Debug, Clone, Copy, Default)]
pub struct LegendSize {
    pub width: f64,
    pub height: f64,
}

/// Specification for a legend, mapping data fields to visual channels.
/// This struct acts as a bridge between data scales and visual guides.
pub struct LegendSpec {
    pub title: String,
    pub field: String,
    pub scale_type: Scale,
    pub domain: ScaleDomain,
    pub has_color: bool,
    pub has_shape: bool,
    pub has_size: bool,
}

impl LegendSpec {
    /// Estimates the size of the legend block with internal column wrapping.
    /// 
    /// This implementation is "Layout-Aware": it respects the `max_h` constraint
    /// provided by the LayoutEngine (which is derived from the Plot Panel height).
    ///
    /// # Arguments
    /// * `theme` - Visual configuration for fonts and spacing.
    /// * `max_h` - The vertical limit for the content. Items will wrap into 
    ///             new columns if they exceed this height.
    pub fn estimate_size(&self, theme: &Theme, max_h: f64) -> LegendSize {
        let font_size = theme.legend_font_size.unwrap_or(theme.tick_label_font_size);
        let title_font_size = font_size * 1.1;

        let title_to_content_gap = theme.legend_title_gap;
        let marker_to_text_gap = theme.legend_marker_text_gap;
        let item_v_gap = theme.legend_item_v_gap; // Vertical spacing between rows
        let col_h_gap = theme.legend_col_h_gap; // Horizontal spacing between wrapped columns

        // 1. Measure Title Dimensions
        let title_w = estimate_text_width(&self.title, title_font_size);
        let title_h = title_font_size;

        // 2. Generate sample labels for measurement
        let labels = self.get_sampling_labels();

        // 3. Measure Content with internal wrapping logic
        let is_discrete = matches!(self.scale_type, Scale::Discrete);
        let use_colorbar = !is_discrete && self.has_color && !self.has_size && !self.has_shape;

        let (content_w, content_h) = if use_colorbar {
            // SCENARIO A: Continuous Colorbar (Gradient Strip)
            // Colorbar height is dynamic but capped to stay proportional to the axis.
            let bar_w = 15.0; 
            let bar_h = f64::min(200.0, max_h * 0.8); 
            let max_lbl_w = labels.iter()
                .map(|l| estimate_text_width(l, font_size))
                .fold(0.0, f64::max);
            (bar_w + marker_to_text_gap + max_lbl_w, bar_h)
        } else {
            // SCENARIO B: Discrete Items or Binned Continuous
            let max_lbl_w = labels.iter()
                .map(|l| estimate_text_width(l, font_size))
                .fold(0.0, f64::max);
            
            let mut total_w = 0.0;
            let mut cur_col_w = 0.0;
            let mut cur_col_h = 0.0;
            let mut max_observed_h = 0.0;

            // Define the "Floor" for content wrapping
            let content_limit = f64::max(max_h - title_h - title_to_content_gap, theme.min_panel_size / 2.0);

            for (i, _) in labels.iter().enumerate() {
                // Determine marker size (grows if 'size' channel is used)
                let marker_w = if self.has_size {
                    if is_discrete { 18.0 } else { 5.0 + (15.0 * (i as f64 / 4.0)) }
                } else { 12.0 };

                let row_h = f64::max(marker_w, font_size);
                let row_w = marker_w + marker_to_text_gap + max_lbl_w;

                // WRAPPING LOGIC: If this row pushes the column over the limit, start a new one.
                if cur_col_h + row_h > content_limit && cur_col_h > 0.0 {
                    total_w += cur_col_w + col_h_gap;
                    max_observed_h = f64::max(max_observed_h, cur_col_h);
                    cur_col_h = row_h;
                    cur_col_w = row_w;
                } else {
                    cur_col_h += row_h;
                    if i < labels.len() - 1 { cur_col_h += item_v_gap; }
                    cur_col_w = f64::max(cur_col_w, row_w);
                }
            }
            total_w += cur_col_w;
            max_observed_h = f64::max(max_observed_h, cur_col_h);
            (total_w, max_observed_h)
        };

        LegendSize {
            width: f64::max(title_w, content_w),
            height: title_h + title_to_content_gap + content_h,
        }
    }

    /// Generates representative labels based on the data domain.
    /// Supports Categorical (all items), Continuous (5 samples), and Temporal (5 samples).
    pub(crate) fn get_sampling_labels(&self) -> Vec<String> {
        match &self.domain {
            ScaleDomain::Categorical(v) => v.clone(),
            
            ScaleDomain::Continuous(min, max) => {
                (0..5).map(|i| {
                    let val = min + (max - min) * (i as f64 / 4.0);
                    format!("{:.2}", val)
                }).collect()
            },
            
            ScaleDomain::Temporal(start, end) => {
                // Using 'time' crate: obtain unix timestamps for consistent math
                let s = start.unix_timestamp();
                let e = end.unix_timestamp();
                let dur = e - s;
                
                (0..5).map(|i| {
                    let tick_ts = s + (dur * i as i64 / 4);
                    // Return raw timestamp string for measurement phase.
                    // Formatting to human-readable strings happens in the renderer.
                    tick_ts.to_string()
                }).collect()
            }
        }
    }
}

/// Management utility to aggregate legend specifications from multiple chart layers.
pub struct LegendManager;

impl LegendManager {
    /// Consolidates aesthetic encodings (color, shape, size) from all layers 
    /// into a unique set of LegendSpecs to avoid redundant legend blocks.
    pub fn collect_legends(layers: &[Box<dyn Layer>]) -> Vec<LegendSpec> {
        let mut specs_map: HashMap<String, LegendSpec> = HashMap::new();
        for layer in layers {
            let enc = layer.get_encoding();
            if let Some(c) = &enc.color { Self::merge_channel(&mut specs_map, &c.field, "color", layer); }
            if let Some(s) = &enc.shape { Self::merge_channel(&mut specs_map, &s.field, "shape", layer); }
            if let Some(z) = &enc.size { Self::merge_channel(&mut specs_map, &z.field, "size", layer); }
        }
        let mut results: Vec<LegendSpec> = specs_map.into_values().collect();
        // Alphabetical sort ensures consistent ordering regardless of layer order
        results.sort_by(|a, b| a.title.cmp(&b.title));
        results
    }

    fn merge_channel(map: &mut HashMap<String, LegendSpec>, field: &str, channel: &str, layer: &Box<dyn Layer>) {
        let entry = map.entry(field.to_string()).or_insert_with(|| LegendSpec {
            title: field.to_string(),
            field: field.to_string(),
            scale_type: layer.get_scale_type(channel).unwrap_or(Scale::Discrete),
            domain: layer.get_domain(channel).unwrap_or(ScaleDomain::Categorical(vec![])),
            has_color: false,
            has_shape: false,
            has_size: false,
        });
        match channel {
            "color" => entry.has_color = true,
            "shape" => entry.has_shape = true,
            "size" => entry.has_size = true,
            _ => {}
        }
    }
}

/// Defines where the legend should be anchored relative to the Plot Panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LegendPosition { Top, Bottom, Left, Right, None }

impl Default for LegendPosition { 
    fn default() -> Self { LegendPosition::Right } 
}