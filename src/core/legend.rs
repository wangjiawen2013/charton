use super::layer::Layer;
use crate::scale::{Scale, ScaleDomain};
use crate::theme::Theme;
use crate::core::utils::estimate_text_width;
use std::collections::HashMap;

/// Represents the physical footprint of a legend component.
#[derive(Debug, Clone, Copy, Default)]
pub struct LegendSize {
    pub width: f64,
    pub height: f64,
}

/// Specification for a legend, mapping data fields to visual channels.
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
    /// Estimates the size of the legend block, considering orientation and wrapping.
    /// 
    /// # Arguments
    /// * `theme` - Visual styling parameters.
    /// * `max_dim` - The primary constraint (Height for Left/Right, Width for Top/Bottom).
    pub fn estimate_size(&self, theme: &Theme, max_dim: f64) -> LegendSize {
        let font_size = theme.legend_label_size.unwrap_or(theme.tick_label_size);
        let title_font_size = font_size * 1.1;
        let title_gap = theme.legend_title_gap;
        let marker_gap = theme.legend_marker_text_gap;
        let item_v_gap = theme.legend_item_v_gap; 
        let col_h_gap = theme.legend_col_h_gap; 

        let title_w = estimate_text_width(&self.title, title_font_size);
        let title_h = title_font_size;

        let labels = self.get_sampling_labels();
        let is_discrete = matches!(self.scale_type, Scale::Discrete);
        let use_colorbar = !is_discrete && self.has_color && !self.has_size && !self.has_shape;

        // Determine orientation context (Heuristic: if max_dim is wide, it's likely horizontal)
        // In a better API, pass LegendPosition explicitly here.
        let is_horizontal_layout = max_dim > 300.0; 

        let (content_w, content_h) = if use_colorbar {
            // --- SCENARIO A: Continuous Colorbar ---
            if is_horizontal_layout {
                // Horizontal Gradient: [Title] [---Bar---]
                let bar_w = 150.0; // Standard width for horizontal colorbars
                let bar_h = 12.0;
                let label_h = font_size + marker_gap;
                (f64::max(title_w, bar_w), bar_h + label_h)
            } else {
                // Vertical Gradient: Title above Bar
                let bar_w = 15.0; 
                let bar_h = f64::min(200.0, max_dim * 0.8); 
                let max_lbl_w = labels.iter()
                    .map(|l| estimate_text_width(l, font_size))
                    .fold(0.0, f64::max);
                (f64::max(title_w, bar_w + marker_gap + max_lbl_w), bar_h)
            }
        } else {
            // --- SCENARIO B: Discrete Items ---
            let max_lbl_w = labels.iter()
                .map(|l| estimate_text_width(l, font_size))
                .fold(0.0, f64::max);
            
            let marker_area_w = 18.0;
            let row_w = marker_area_w + marker_gap + max_lbl_w;
            let row_h = f64::max(marker_area_w, font_size);

            if is_horizontal_layout {
                // Horizontal Wrapping: items flow Left -> Right, then Wrap to New Row
                let mut total_h = row_h;
                let mut cur_row_w = 0.0;
                let mut max_observed_w = 0.0;

                for (i, _) in labels.iter().enumerate() {
                    if cur_row_w + row_w > max_dim && i > 0 {
                        total_h += row_h + item_v_gap;
                        max_observed_w = f64::max(max_observed_w, cur_row_w);
                        cur_row_w = row_w;
                    } else {
                        cur_row_w += row_w;
                        if i < labels.len() - 1 { cur_row_w += col_h_gap; }
                    }
                }
                (f64::max(title_w, f64::max(max_observed_w, cur_row_w)), total_h)
            } else {
                // Vertical Wrapping: items flow Top -> Bottom, then Wrap to New Column
                let content_limit = f64::max(max_dim - title_h - title_gap, 50.0);
                let mut total_w = 0.0;
                let mut cur_col_h = 0.0;
                let mut cur_col_w = 0.0;
                let mut max_observed_h = 0.0;

                for (i, _) in labels.iter().enumerate() {
                    if cur_col_h + row_h > content_limit && i > 0 {
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
                (f64::max(title_w, total_w), max_observed_h)
            }
        };

        LegendSize {
            width: content_w,
            height: title_h + title_gap + content_h,
        }
    }

    /// Generates sample labels for size estimation.
    pub(crate) fn get_sampling_labels(&self) -> Vec<String> {
        match &self.domain {
            ScaleDomain::Categorical(v) => v.clone(),
            ScaleDomain::Continuous(min, max) => {
                (0..5).map(|i| {
                    let val = min + (max - min) * (i as f64 / 4.0);
                    if val.abs() > 1000.0 || (val.abs() < 0.01 && val != 0.0) {
                        format!("{:.1e}", val)
                    } else {
                        format!("{:.2}", val)
                    }
                }).collect()
            },
            ScaleDomain::Temporal(start, end) => {
                let s = start.unix_timestamp();
                let e = end.unix_timestamp();
                let dur = e - s;
                (0..5).map(|i| (s + (dur * i as i64 / 4)).to_string()).collect()
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