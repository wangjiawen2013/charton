//! Wrap faceting implementation.
//!
//! This module provides the concrete implementation of the `Facet` trait
//! for wrap layouts (single field, wrapped into a 2D grid).

use crate::coordinate::Rect;
use crate::facets::{
    Facet, FacetGridGeometry, FacetMetrics, FacetPanel, FacetPanelInfo, FacetStrategy,
};
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
        metrics: &FacetMetrics,
        theme: &Theme,
    ) -> Vec<FacetPanel> {
        let values = &factors[0];
        let n = values.len();

        // --- Pass 1: grid shape -------------------------------------------------
        // The number of columns is explicit or the square root of the panel
        // count; the last row may therefore be partial.
        let cols = self
            .columns
            .unwrap_or_else(|| (n as f64).sqrt().ceil() as usize)
            .max(1);
        let rows = n.div_ceil(cols).max(1);

        // --- Pass 2: decide which axis tracks the grid needs --------------------
        //
        // Same idea as the grid facet, with one extra wrinkle: because the last
        // row can be incomplete, "the bottom-most cell of a column" is not always
        // in the last row. `is_last_in_column` tells `axis_visibility` which cell
        // sits at the bottom of its own column, so an x axis is still labelled on
        // a column that ends early. A column with no cell at all never asks for a
        // track.
        let visibility = |idx: usize| {
            let r = idx / cols;
            let c = idx % cols;
            let is_last_in_column = idx + cols >= n;
            self.strategy.axis_visibility(r, c, rows, is_last_in_column)
        };
        let mut has_left_axis = vec![false; cols];
        let mut has_bottom_axis = vec![false; rows];
        for idx in 0..n {
            let r = idx / cols;
            let c = idx % cols;
            let (show_x_axis, show_y_axis) = visibility(idx);
            // A track serves the whole column/row, so one demanding cell reserves it.
            has_left_axis[c] |= show_y_axis;
            has_bottom_axis[r] |= show_x_axis;
        }

        // --- Pass 3: solve the tracks and place the panels ----------------------
        // Reserving `cols` tracks even for a partial last row keeps panels across
        // rows aligned in the same columns.
        let geometry = FacetGridGeometry::new(
            rows,
            cols,
            container,
            metrics,
            theme,
            &has_left_axis,
            &has_bottom_axis,
        );

        values
            .iter()
            .enumerate()
            .map(|(idx, val)| {
                let r = idx / cols;
                let c = idx % cols;
                let (show_x_axis, show_y_axis) = visibility(idx);

                FacetPanel {
                    rect: geometry.panel_rect(r, c),
                    header_rect: geometry.header_rect(r, c),
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
