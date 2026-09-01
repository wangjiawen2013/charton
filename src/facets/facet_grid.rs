//! Grid faceting implementation.
//!
//! This module provides the concrete implementation of the `Facet` trait
//! for grid layouts (row field + column field, strict matrix layout).

use crate::coordinate::Rect;
use crate::facets::{Facet, FacetPanel, FacetPanelInfo, FacetStrategy};
use crate::theme::Theme;

/// Internal implementation of Grid faceting.
///
/// This is the concrete type that implements the `Facet` trait.
/// Users should use `FacetSpec` to create facets instead of constructing
/// this directly.
#[derive(Debug, Clone)]
pub struct FacetGridImpl {
    pub row_field: String,
    pub col_field: String,
    pub strategy: FacetStrategy,
}

impl FacetGridImpl {
    /// Creates a new Grid facet with row and column fields.
    pub const fn new(row_field: String, col_field: String) -> Self {
        Self {
            row_field,
            col_field,
            strategy: FacetStrategy::Fixed,
        }
    }

    /// Sets the scale strategy for the facet.
    pub const fn with_strategy(mut self, strategy: FacetStrategy) -> Self {
        self.strategy = strategy;
        self
    }
}

impl Facet for FacetGridImpl {
    fn fields(&self) -> Vec<String> {
        vec![self.row_field.clone(), self.col_field.clone()]
    }

    fn strategy(&self) -> FacetStrategy {
        self.strategy
    }

    fn compute_panels(
        &self,
        factors: &[Vec<String>],
        container: &Rect,
        theme: &Theme,
    ) -> Vec<FacetPanel> {
        let row_values = &factors[0];
        let col_values = &factors[1];

        let n_rows = row_values.len().max(1);
        let n_cols = col_values.len().max(1);

        // 1. Calculate panel and plot dimensions
        let gap = theme.facet_spacing;
        let header_h = theme.facet_label_size * 1.5 + theme.facet_strip_padding * 2.0;

        let panel_w = (container.width - (n_cols - 1) as f64 * gap) / n_cols as f64;
        let panel_h = (container.height - (n_rows - 1) as f64 * gap) / n_rows as f64;

        let axis_pad = (theme.label_size + theme.tick_label_size + 12.0).max(24.0);
        let plot_w = (panel_w - axis_pad - 12.0).max(40.0);
        let plot_h = (panel_h - header_h - 8.0 - axis_pad).max(40.0);

        // 2. Generate panel layouts using functional style
        row_values
            .iter()
            .enumerate()
            .flat_map(|(r_idx, r_val)| {
                col_values.iter().enumerate().map(move |(c_idx, c_val)| {
                    let x = container.x + c_idx as f64 * (panel_w + gap);
                    let header_y = container.y + r_idx as f64 * (panel_h + gap);

                    FacetPanel {
                        rect: Rect::new(x + axis_pad, header_y + header_h + 8.0, plot_w, plot_h),
                        header_rect: Rect::new(x, header_y, panel_w, header_h),
                        info: FacetPanelInfo {
                            row: r_idx,
                            col: c_idx,
                            total_rows: n_rows,
                            total_cols: n_cols,
                            label: format!("{} | {}", r_val, c_val),
                            row_label: r_val.clone(),
                            col_label: c_val.clone(),
                            facet_filter: vec![
                                (self.row_field.clone(), r_val.clone()),
                                (self.col_field.clone(), c_val.clone()),
                            ],
                        },
                    }
                })
            })
            .collect()
    }
}
