use crate::scale::mapper::VisualMapper;
use crate::scale::ScaleDomain;
use crate::scale::Tick;
use crate::theme::Theme;
use crate::core::utils::estimate_text_width;
use crate::core::aesthetics::{GlobalAesthetics, AestheticMapping};
use std::collections::BTreeMap;

/// Represents the physical rectangular area required by a Guide (Legend or ColorBar).
/// Used by the LayoutEngine to reserve space and calculate the final Plot Panel.
#[derive(Debug, Clone, Copy, Default)]
pub struct GuideSize {
    pub width: f32,
    pub height: f32,
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
            if let Some(mapper) = m.scale_impl.mapper() {
                match mapper {
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
    pub fn estimate_size(&self, theme: &Theme, max_h: f32) -> GuideSize {
        match self.kind {
            GuideKind::ColorBar => self.estimate_colorbar_size(theme, max_h),
            GuideKind::Legend => self.estimate_legend_size(theme, max_h),
        }
    }

    /// Estimates dimensions for a gradient ColorBar.
    fn estimate_colorbar_size(&self, theme: &Theme, max_h: f32) -> GuideSize {
        let font_size = theme.legend_label_size;
        let title_font_size = font_size * 1.1;
        
        let title_w = estimate_text_width(&self.title, title_font_size);
        let bar_w = 15.0; // Standard thickness of the color strip
        
        let labels = self.get_sampling_labels();
        let max_lbl_w = labels.iter()
            .map(|l| estimate_text_width(l, font_size))
            .fold(0.0, f32::max);

        GuideSize {
            width: f32::max(title_w, bar_w + theme.legend_marker_text_gap + max_lbl_w),
            // Height is usually 70% of plot height or capped at a reasonable max (200px)
            height: title_font_size + theme.legend_title_gap + f32::min(200.0, max_h * 0.7),
        }
    }

    /// Estimates dimensions for a discrete Legend, supporting multi-column wrapping.
    fn estimate_legend_size(&self, theme: &Theme, max_h: f32) -> GuideSize {
        let font_size = theme.legend_label_size;
        let title_font_size = font_size * 1.1;

        let title_w = estimate_text_width(&self.title, title_font_size);
        let title_h = title_font_size;

        let labels = self.get_sampling_labels();
        let max_lbl_w = labels.iter()
            .map(|l| estimate_text_width(l, font_size))
            .fold(0.0, f32::max);
        
        let mut total_w = 0.0;
        let mut cur_col_w = 0.0;
        let mut cur_col_h = 0.0;
        let mut max_observed_h = 0.0;

        // Content area is limited by the total plot height minus the title space
        let content_limit = f32::max(max_h - title_h - theme.legend_title_gap, 20.0);

        for (i, _) in labels.iter().enumerate() {
            let marker_area_w = 18.0; // Reserved square for the icon/glyph
            let row_h = f32::max(marker_area_w, font_size);
            let row_w = marker_area_w + theme.legend_marker_text_gap + max_lbl_w;

            // Column Wrapping Logic: Start a new column if the current one is full
            if cur_col_h + row_h > content_limit && cur_col_h > 0.0 {
                total_w += cur_col_w + theme.legend_col_h_gap;
                max_observed_h = f32::max(max_observed_h, cur_col_h);
                cur_col_h = row_h;
                cur_col_w = row_w;
            } else {
                cur_col_h += row_h;
                if i < labels.len() - 1 { cur_col_h += theme.legend_item_v_gap; }
                cur_col_w = f32::max(cur_col_w, row_w);
            }
        }
        
        total_w += cur_col_w;
        max_observed_h = f32::max(max_observed_h, cur_col_h);

        GuideSize {
            width: f32::max(title_w, total_w),
            height: title_h + theme.legend_title_gap + max_observed_h,
        }
    }

    /// Extracts string labels from the underlying Scale implementation and 
    /// enforces uniform decimal precision for visual alignment.
    /// 
    /// This method ensures that all labels in a legend block share the same number 
    /// of decimal places, preventing jagged text alignment (e.g., ensuring "20.0" 
    /// isn't shortened to "20" when appearing alongside "16.3").
    pub(crate) fn get_sampling_labels(&self) -> Vec<String> {
        if let Some(first_mapping) = self.mappings.first() {
            // 1. Define target density (e.g., we want 5 circles for Size)
            let count = match self.kind {
                GuideKind::ColorBar => 5,
                GuideKind::Legend => {
                    if let ScaleDomain::Discrete(ref v) = self.domain { v.len() } else { 5 }
                }
            };

            // 2. Retrieve raw ticks from the scale (Pretty algorithm or Sample_n)
            let mut ticks = first_mapping.scale_impl.ticks(count);

            // Fallback to force-sampling if the pretty algorithm returns insufficient points
            if ticks.len() < 3 && !matches!(self.domain, ScaleDomain::Discrete(_)) {
                ticks = first_mapping.scale_impl.sample_n(count);
            }

            // 3. --- Uniform Precision Logic ---
            
            // Check if we are dealing with a numeric (non-categorical) scale
            if !matches!(self.domain, ScaleDomain::Discrete(_)) {
                // Determine the maximum precision needed across all sampled points.
                // We look for the most specific decimal place to ensure no data is lost.
                let mut max_precision = 0;
                let has_fractions = ticks.iter().any(|t| (t.value - t.value.floor()).abs() > 1e-9);

                if has_fractions {
                    for tick in &ticks {
                        // Find how many decimals this specific number actually uses
                        let s = format!("{}", tick.value);
                        if let Some(pos) = s.find('.') {
                            let p = s.len() - pos - 1;
                            if p > max_precision { max_precision = p; }
                        }
                    }
                    // For aesthetics, we force at least 1 decimal if any fractions exist
                    max_precision = max_precision.max(1).min(4);
                }

                // Re-format all ticks using the discovered global precision
                ticks.into_iter().map(|t| {
                    format!("{:.1$}", t.value, max_precision)
                }).collect()
            } else {
                // For categorical data, use labels exactly as provided by the scale
                ticks.into_iter().map(|t| t.label).collect()
            }
        } else {
            // Fallback for empty mappings
            match &self.domain {
                ScaleDomain::Discrete(v) => v.clone(),
                _ => Vec::new(),
            }
        }
    }

    /// Returns the raw Tick objects (value + aligned label) used for sampling.
    pub(crate) fn get_sampling_ticks(&self) -> Vec<Tick> {
        if let Some(first_mapping) = self.mappings.first() {
            let count = 5; // Target density
            let mut ticks = first_mapping.scale_impl.ticks(count);
            
            if ticks.len() < 3 && !matches!(self.domain, ScaleDomain::Discrete(_)) {
                ticks = first_mapping.scale_impl.sample_n(count);
            }

            // Apply the precision alignment we discussed earlier
            let mut max_p = 0;
            let has_fractions = ticks.iter().any(|t| (t.value - t.value.floor()).abs() > 1e-9);
            if has_fractions {
                for t in &ticks {
                    let s = format!("{}", t.value);
                    if let Some(pos) = s.find('.') {
                        max_p = max_p.max(s.len() - pos - 1);
                    }
                }
                max_p = max_p.max(1).min(4);
            }

            // Update labels in the ticks themselves
            for t in &mut ticks {
                t.label = format!("{:.1$}", t.value, max_p);
            }
            ticks
        } else {
            Vec::new()
        }
    }
}

/// Core manager responsible for grouping aesthetics and generating GuideSpecs.
pub struct GuideManager;

impl GuideManager {
    /// Orchestrates the collection of global aesthetics into a consolidated set of GuideSpecs.
    /// 
    /// This function implements the "Legend Merging" logic. According to the Grammar of Graphics, 
    /// if multiple aesthetics (e.g., Color, Shape, and Size) are mapped to the same data field, 
    /// they should be unified into a single visual guide (Legend) to avoid redundancy and 
    /// improve scannability.
    ///
    /// # Logic Flow:
    /// 1. Group all active `AestheticMapping` instances by their `field` name.
    /// 2. Use a `BTreeMap` to ensure that guides are generated in a stable, alphabetical order.
    /// 3. Pass the consolidated mappings to `GuideSpec::new`, which infers the visual 
    ///    type (Legend vs. ColorBar) based on the combined mapping properties.
    pub fn collect_guides(aesthetics: &GlobalAesthetics) -> Vec<GuideSpec> {
        // We group mappings by field name. The tuple contains the inferred ScaleDomain 
        // and the list of mappings associated with that field.
        let mut field_map: BTreeMap<String, (ScaleDomain, Vec<AestheticMapping>)> = BTreeMap::new();

        // Helper closure to safely extract and group active mappings.
        let mut collect = |mapping: &Option<AestheticMapping>| {
            if let Some(m) = mapping {
                let entry = field_map.entry(m.field.clone()).or_insert_with(|| {
                    // We capture the domain from the first mapping encountered for this field.
                    // In a valid plot, all aesthetics sharing a field should share the same scale logic.
                    (m.scale_impl.get_domain_enum(), Vec::new())
                });
                entry.1.push(m.clone());
            }
        };

        // --- Phase 1: Aggregation ---
        // Scan standard aesthetic channels. Order of collection doesn't affect the 
        // result because BTreeMap handles the final sorting.
        collect(&aesthetics.color);
        collect(&aesthetics.shape);
        collect(&aesthetics.size);

        // --- Phase 2: Specification ---
        // Convert each field group into a high-level GuideSpec.
        // The GuideSpec will later use the `sample_n` logic implemented in the scales 
        // to generate the 5 visual steps (circles/colors) you requested.
        field_map
            .into_iter()
            .map(|(field, (domain, mappings))| {
                // GuideSpec::new performs semantic inference to decide if this 
                // should be rendered as a discrete Legend or a continuous ColorBar.
                GuideSpec::new(field, domain, mappings)
            })
            .collect()
    }
}

/// Defines where the legend block is placed relative to the chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LegendPosition { Top, Bottom, Left, Right, None }

impl Default for LegendPosition { 
    fn default() -> Self { LegendPosition::Right } 
}