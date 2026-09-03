//! Wrap faceting implementation.
//!
//! This module provides the concrete implementation of the `Facet` trait
//! for wrap layouts (single field, wrapped into a 2D grid).

use crate::coordinate::Rect;
use crate::facets::{Facet, FacetPanel, FacetPanelInfo, FacetStrategy};
use crate::theme::Theme;

/// Internal implementation of Wrap faceting.
///
/// This is the concrete type that implements the `Facet` trait.
/// Users should use `FacetSpec` to create facets instead of constructing
/// this directly.
#[derive(Debug, Clone)]
pub struct FacetWrapImpl {
    pub field: String,
    pub columns: Option<usize>,
    pub strategy: FacetStrategy,
}

impl FacetWrapImpl {
    /// Creates a new Wrap facet with the given field.
    pub const fn new(field: String) -> Self {
        Self {
            field,
            columns: None,
            strategy: FacetStrategy::Fixed,
        }
    }

    /// Sets the number of columns in the grid.
    pub const fn with_columns(mut self, columns: Option<usize>) -> Self {
        self.columns = columns;
        self
    }

    /// Sets the scale strategy for the facet.
    pub const fn with_strategy(mut self, strategy: FacetStrategy) -> Self {
        self.strategy = strategy;
        self
    }
}

impl Facet for FacetWrapImpl {
    fn fields(&self) -> Vec<String> {
        vec![self.field.clone()]
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
        let values = &factors[0];
        let n = values.len();

        // 1. Calculate grid dimensions (columns and rows)
        let cols = self
            .columns
            .unwrap_or_else(|| (n as f64).sqrt().ceil() as usize)
            .max(1);
        let rows = n.div_ceil(cols);

        // 2. Calculate panel and plot dimensions
        let gap = theme.facet_spacing;
        let header_h = theme.facet_label_size * 1.5 + theme.facet_strip_padding * 2.0;

        let panel_w = (container.width - (cols - 1) as f64 * gap) / cols as f64;
        let panel_h = (container.height - (rows - 1) as f64 * gap) / rows as f64;

        let axis_pad = (theme.label_size + theme.tick_label_size + 12.0).max(24.0);

        // 3. Generate panel layouts
        values
            .iter()
            .enumerate()
            .map(|(idx, val)| {
                let r = idx / cols;
                let c = idx % cols;
                let is_last_in_column = idx + cols >= n;
                let (show_x_axis, show_y_axis) =
                    self.strategy.axis_visibility(r, c, rows, is_last_in_column);

                let x = container.x + c as f64 * (panel_w + gap);
                let header_y = container.y + r as f64 * (panel_h + gap);

                FacetPanel {
                    rect: Rect::new(
                        x + axis_pad,
                        header_y + header_h + 8.0,
                        (panel_w - axis_pad - 12.0).max(40.0),
                        (panel_h - header_h - 8.0 - axis_pad).max(40.0),
                    ),
                    header_rect: Rect::new(x, header_y, panel_w, header_h),
                    info: FacetPanelInfo {
                        row: r,
                        col: c,
                        total_rows: rows,
                        total_cols: cols,
                        label: format!("{} = {}", self.field, val),
                        row_label: val.clone(),
                        col_label: String::new(),
                        facet_filter: vec![(self.field.clone(), val.clone())],
                        show_x_axis,
                        show_y_axis,
                    },
                }
            })
            .collect()
    }
}
