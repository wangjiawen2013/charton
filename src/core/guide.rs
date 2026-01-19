use crate::scale::mapper::VisualMapper;
use crate::scale::ScaleDomain;
use crate::theme::Theme;
use crate::core::utils::estimate_text_width;
use crate::core::aesthetics::{GlobalAesthetics, AestheticMapping};
use std::collections::BTreeMap;

/// Represents the physical rectangular area required by a Guide (Legend or ColorBar).
/// Used by the LayoutEngine to reserve space and calculate the final Plot Panel.
#[derive(Debug, Clone, Copy, Default)]
pub struct GuideSize {
    pub width: f64,
    pub height: f64,
}

/// The visual representation strategy for a data field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuideKind {
    /// A discrete list of symbols and labels. Used for categorical data, 
    /// or when multiple aesthetics (e.g., Color + Shape) are merged.
    Legend,
    /// A continuous gradient strip. Used exclusively for continuous Color mappings.
    ColorBar,
}

/// Specification for a Guide (Legend or ColorBar), acting as the bridge 
/// between abstract data scales and visual rendering instructions.
///
/// Following the "Grammar of Graphics" (like ggplot2), a single GuideSpec 
/// consolidates multiple aesthetics (Color, Shape, Size) if they map to the same field.
pub struct GuideSpec {
    /// The title displayed above the guide (usually the data field name).
    pub title: String,
    /// The data field name this guide represents (e.g., "mpg", "class").
    pub field: String,
    /// Determines if this is rendered as a discrete list or a gradient bar.
    pub kind: GuideKind,
    /// The data range and type (Categorical or Continuous).
    pub domain: ScaleDomain,
    /// The collection of visual mappings tied to this specific field.
    pub mappings: Vec<AestheticMapping>,
}

impl GuideSpec {
    /// Constructs a GuideSpec and performs **Semantic Inference**:
    /// 1. If any mapping involves Size or Shape, it is forced to be a `Legend`.
    /// 2. Only if it is strictly a continuous Color mapping does it become a `ColorBar`.
    pub fn new(field: String, domain: ScaleDomain, mappings: Vec<AestheticMapping>) -> Self {
        let mut has_complex_geometry = false;
        let mut is_continuous_color = false;

        for m in &mappings {
            match &m.mapper {
                // Size and Shape require discrete symbol keys
                VisualMapper::Size { .. } | VisualMapper::Shape { .. } => {
                    has_complex_geometry = true;
                }
                // Continuous color can potentially use a gradient bar
                VisualMapper::ContinuousColor { .. } => {
                    is_continuous_color = true;
                }
                _ => {}
            }
        }

        // If it involves symbols (Shape/Size) or mixed channels, we use Legend mode.
        // ColorBar is reserved for pure continuous color mapping.
        let kind = if is_continuous_color && !has_complex_geometry {
            GuideKind::ColorBar
        } else {
            GuideKind::Legend
        };

        Self {
            title: field.clone(),
            field,
            kind,
            domain,
            mappings,
        }
    }

    /// Entry point for the LayoutEngine to calculate required pixels.
    pub fn estimate_size(&self, theme: &Theme, max_h: f64) -> GuideSize {
        match self.kind {
            GuideKind::ColorBar => self.estimate_colorbar_size(theme, max_h),
            GuideKind::Legend => self.estimate_legend_size(theme, max_h),
        }
    }

    /// Estimates dimensions for a gradient ColorBar.
    fn estimate_colorbar_size(&self, theme: &Theme, max_h: f64) -> GuideSize {
        let font_size = theme.legend_label_size.unwrap_or(theme.tick_label_size);
        let title_font_size = font_size * 1.1;
        
        let title_w = estimate_text_width(&self.title, title_font_size);
        let bar_w = 15.0; // Standard thickness of the color strip
        
        let labels = self.get_sampling_labels();
        let max_lbl_w = labels.iter()
            .map(|l| estimate_text_width(l, font_size))
            .fold(0.0, f64::max);

        GuideSize {
            width: f64::max(title_w, bar_w + theme.legend_marker_text_gap + max_lbl_w),
            // Height is usually 70% of plot height or capped at a reasonable max (200px)
            height: title_font_size + theme.legend_title_gap + f64::min(200.0, max_h * 0.7),
        }
    }

    /// Estimates dimensions for a discrete Legend, supporting multi-column wrapping.
    fn estimate_legend_size(&self, theme: &Theme, max_h: f64) -> GuideSize {
        let font_size = theme.legend_label_size.unwrap_or(theme.tick_label_size);
        let title_font_size = font_size * 1.1;

        let title_w = estimate_text_width(&self.title, title_font_size);
        let title_h = title_font_size;

        let labels = self.get_sampling_labels();
        let max_lbl_w = labels.iter()
            .map(|l| estimate_text_width(l, font_size))
            .fold(0.0, f64::max);
        
        let mut total_w = 0.0;
        let mut cur_col_w = 0.0;
        let mut cur_col_h = 0.0;
        let mut max_observed_h = 0.0;

        // Content area is limited by the total plot height minus the title space
        let content_limit = f64::max(max_h - title_h - theme.legend_title_gap, 20.0);

        for (i, _) in labels.iter().enumerate() {
            let marker_area_w = 18.0; // Reserved square for the icon/glyph
            let row_h = f64::max(marker_area_w, font_size);
            let row_w = marker_area_w + theme.legend_marker_text_gap + max_lbl_w;

            // Column Wrapping Logic: Start a new column if the current one is full
            if cur_col_h + row_h > content_limit && cur_col_h > 0.0 {
                total_w += cur_col_w + theme.legend_col_h_gap;
                max_observed_h = f64::max(max_observed_h, cur_col_h);
                cur_col_h = row_h;
                cur_col_w = row_w;
            } else {
                cur_col_h += row_h;
                if i < labels.len() - 1 { cur_col_h += theme.legend_item_v_gap; }
                cur_col_w = f64::max(cur_col_w, row_w);
            }
        }
        
        total_w += cur_col_w;
        max_observed_h = f64::max(max_observed_h, cur_col_h);

        GuideSize {
            width: f64::max(title_w, total_w),
            height: title_h + theme.legend_title_gap + max_observed_h,
        }
    }

    /// Extracts string labels from the underlying Scale implementation.
    /// This ensures legend labels match axis labels (e.g., date formatting).
    pub(crate) fn get_sampling_labels(&self) -> Vec<String> {
        if let Some(first_mapping) = self.mappings.first() {
            let count = match self.kind {
                GuideKind::ColorBar => 5,
                GuideKind::Legend => {
                    if let ScaleDomain::Categorical(ref v) = self.domain { v.len() } else { 5 }
                }
            };

            // Delegate to the ScaleTrait's tick generation
            first_mapping.scale_impl.ticks(count)
                .into_iter()
                .map(|t| t.label)
                .collect()
        } else {
            match &self.domain {
                ScaleDomain::Categorical(v) => v.clone(),
                _ => Vec::new(),
            }
        }
    }
}

/// Core manager responsible for grouping aesthetics and generating GuideSpecs.
pub struct GuideManager;

impl GuideManager {
    /// Collects and merges aesthetic mappings into a list of GuideSpecs.
    /// 
    /// This implementation uses `get_domain_enum()` from the ScaleTrait 
    /// to ensure the GuideSpec has the correct ScaleDomain (Categorical, Continuous, or Temporal).
    pub fn collect_guides(aesthetics: &GlobalAesthetics) -> Vec<GuideSpec> {
        // We use a BTreeMap to group mappings by their field name (e.g., "mpg").
        // BTreeMap ensures the legends are ordered alphabetically by field name.
        let mut field_map: BTreeMap<String, Vec<AestheticMapping>> = BTreeMap::new();

        // 1. Group active mappings by their source field
        if let Some(ref m) = aesthetics.color { 
            field_map.entry(m.field.clone()).or_default().push(m.clone()); 
        }
        if let Some(ref m) = aesthetics.shape { 
            field_map.entry(m.field.clone()).or_default().push(m.clone()); 
        }
        if let Some(ref m) = aesthetics.size { 
            field_map.entry(m.field.clone()).or_default().push(m.clone()); 
        }

        // 2. Create GuideSpecs for each unique field
        let mut results: Vec<GuideSpec> = field_map.into_iter().map(|(field, mappings)| {
            // Because all mappings for the same field share the same underlying data,
            // we can safely pull the domain information from the first mapping.
            // Using the trait's provided helper to get the full ScaleDomain enum.
            let domain = mappings[0].scale_impl.get_domain_enum(); 
            
            // GuideSpec::new will then perform Semantic Inference to decide 
            // if this should be a Legend or a ColorBar.
            GuideSpec::new(field, domain, mappings)
        }).collect();

        // 3. Final Sort (though BTreeMap already handled grouping, 
        // this ensures the result vector is stable).
        results.sort_by(|a, b| a.field.cmp(&b.field));
        
        results
    }
}

/// Defines where the legend block is placed relative to the chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LegendPosition { Top, Bottom, Left, Right, None }

impl Default for LegendPosition { 
    fn default() -> Self { LegendPosition::Right } 
}