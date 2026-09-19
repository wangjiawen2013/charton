//! Grid faceting implementation.
//!
//! This module provides the concrete implementation of the `Facet` trait
//! for grid layouts (row field + column field, strict matrix layout).

use crate::coordinate::Rect;
use crate::facets::{
    Facet, FacetGridGeometry, FacetMetrics, FacetPanel, FacetPanelInfo, FacetStrategy,
};
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
        metrics: &FacetMetrics,
        theme: &Theme,
    ) -> Vec<FacetPanel> {
        let row_values = &factors[0];
        let col_values = &factors[1];

        // A grid is a full matrix: there is a cell for every (row, column)
        // combination, even when that combination holds no data.
        let n_rows = row_values.len().max(1);
        let n_cols = col_values.len().max(1);

        // --- Pass 1: decide which axis tracks the grid needs --------------------
        //
        // The grid owns the axis space, so it has to know which columns draw a y
        // axis and which rows draw an x axis *before* it can size the panels. The
        // per-cell decision is delegated to `axis_visibility`; here it is folded
        // into per-track flags, because an axis track belongs to the whole column
        // or row, not to one cell: any cell asking for the axis is enough to
        // reserve the track for the entire track.
        //
        // `Fixed` asks for the y axis only in column 0 and the x axis only in the
        // last row, so just those two tracks are created. `Free` asks for both in
        // every cell, so every column and every row gets a track.
        let mut has_left_axis = vec![false; n_cols];
        let mut has_bottom_axis = vec![false; n_rows];
        for (r_idx, row_axis) in has_bottom_axis.iter_mut().enumerate() {
            for (c_idx, col_axis) in has_left_axis.iter_mut().enumerate() {
                let (show_x_axis, show_y_axis) =
                    self.strategy.axis_visibility(r_idx, c_idx, n_rows, false);
                *col_axis |= show_y_axis;
                *row_axis |= show_x_axis;
            }
        }

        // --- Pass 2: solve the tracks and place the panels ----------------------
        //
        // `FacetGridGeometry` turns the flags into one shared panel size plus the
        // cumulative offset of every track; this pass only reads those back, so
        // the laid-out rects can never disagree with the space that was reserved.
        let geometry = FacetGridGeometry::new(
            n_rows,
            n_cols,
            container,
            metrics,
            theme,
            &has_left_axis,
            &has_bottom_axis,
        );

        // Borrowed so the `move` closure below captures a reference, not the grid
        // itself (which is not `Copy`).
        let geometry = &geometry;
        row_values
            .iter()
            .enumerate()
            .flat_map(|(r_idx, r_val)| {
                col_values.iter().enumerate().map(move |(c_idx, c_val)| {
                    let (show_x_axis, show_y_axis) =
                        self.strategy.axis_visibility(r_idx, c_idx, n_rows, false);

                    FacetPanel {
                        rect: geometry.panel_rect(r_idx, c_idx),
                        header_rect: geometry.header_rect(r_idx, c_idx),
                        info: FacetPanelInfo {
                            row: r_idx,
                            col: c_idx,
                            total_rows: n_rows,
                            total_cols: n_cols,
                            label: format!(
                                "{} = {} | {} = {}",
                                self.row_field, r_val, self.col_field, c_val
                            ),
                            row_label: r_val.clone(),
                            col_label: c_val.clone(),
                            facet_filter: vec![
                                (self.row_field.clone(), r_val.clone()),
                                (self.col_field.clone(), c_val.clone()),
                            ],
                            show_x_axis,
                            show_y_axis,
                        },
                    }
                })
            })
            .collect()
    }
}
