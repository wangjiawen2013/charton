//! Faceting module for creating multi-panel visualizations.
//!
//! This module provides the core infrastructure for splitting charts into
//! multiple panels based on data fields.

mod facet_grid;
mod facet_wrap;

use crate::coordinate::Rect;
pub use facet_grid::FacetGridImpl;
pub use facet_wrap::FacetWrapImpl;

// ============== Core Facet Trait ==============

/// The core trait that all Faceting methods must implement.
///
/// This allows the rendering engine to treat different facet types
/// (Wrap, Grid, etc.) polymorphically.
pub trait Facet: Send + Sync {
    /// Returns the data column(s) required for faceting.
    fn fields(&self) -> Vec<String>;

    /// Returns the scale resolution strategy (Fixed vs Free).
    fn strategy(&self) -> FacetStrategy;

    /// Computes the physical panel layout for the facets.
    ///
    /// # Arguments
    /// * `factors` - The unique values from the data fields, in the same order
    ///   as returned by `fields()`.
    /// * `container` - The total area available for all facets.
    /// * `theme` - Theme settings for spacing and label sizes.
    ///
    /// # Returns
    /// A `Vec<FacetPanel>` ordered row-major. Each panel contains its plot
    /// rectangle, header rectangle, and facet filter for data subsetting.
    fn compute_panels(
        &self,
        factors: &[Vec<String>],
        container: &Rect,
        theme: &crate::theme::Theme,
    ) -> Vec<FacetPanel>;
}

// ============== User-Friendly API Entry Point ==============

/// User-friendly specification for creating facets.
///
/// This enum provides a concise way to define faceting without needing
/// to construct `FacetWrap` or `FacetGrid` directly.
#[derive(Debug, Clone)]
pub enum FacetSpec {
    /// Wrap layout: panels arranged in a 2D grid with configurable columns.
    Wrap {
        /// The data field used to split the data into panels.
        field: String,
        /// Number of columns. If `None`, automatically determined.
        columns: Option<usize>,
        /// Scale strategy for the facets.
        strategy: FacetStrategy,
    },
    /// Grid layout: panels arranged in a 2D matrix using two fields.
    Grid {
        /// The data field used to split the data into rows.
        row_field: String,
        /// The data field used to split the data into columns.
        col_field: String,
        /// Scale strategy for the facets.
        strategy: FacetStrategy,
    },
}

impl FacetSpec {
    /// Creates a Wrap facet specification with default settings.
    pub fn wrap(field: &str) -> Self {
        FacetSpec::Wrap {
            field: field.to_string(),
            columns: None,
            strategy: FacetStrategy::Fixed,
        }
    }

    /// Creates a Grid facet specification with default settings.
    pub fn grid(row_field: &str, col_field: &str) -> Self {
        FacetSpec::Grid {
            row_field: row_field.to_string(),
            col_field: col_field.to_string(),
            strategy: FacetStrategy::Fixed,
        }
    }

    /// Fluent builder method to set explicit column count (only applies to Wrap layout).
    pub const fn with_columns(mut self, cols: usize) -> Self {
        if let FacetSpec::Wrap {
            ref mut columns, ..
        } = self
        {
            *columns = Some(cols);
        }
        self
    }

    /// Fluent builder method to set the axes sharing strategy.
    ///
    /// Accepts `FacetStrategy` variants or string literals like `"free"`, `"free_x"`.
    pub fn with_strategy(mut self, strategy: impl Into<FacetStrategy>) -> Self {
        let strat = strategy.into();
        match self {
            FacetSpec::Wrap {
                strategy: ref mut s,
                ..
            }
            | FacetSpec::Grid {
                strategy: ref mut s,
                ..
            } => {
                *s = strat;
            }
        }
        self
    }

    /// Converts this specification into a concrete `Facet` implementation.
    ///
    /// Used internally by `LayeredChart` to perform physical layout calculations.
    pub fn into_facet(self) -> Box<dyn Facet> {
        match self {
            FacetSpec::Wrap {
                field,
                columns,
                strategy,
            } => {
                let wrap = FacetWrapImpl::new(field)
                    .with_columns(columns)
                    .with_strategy(strategy);
                Box::new(wrap)
            }
            FacetSpec::Grid {
                row_field,
                col_field,
                strategy,
            } => {
                let grid = FacetGridImpl::new(row_field, col_field).with_strategy(strategy);
                Box::new(grid)
            }
        }
    }
}

// ============== Convenient From Implementations ==============

/// Enables `chart.facet("category")`
impl From<&str> for FacetSpec {
    fn from(field: &str) -> Self {
        FacetSpec::wrap(field)
    }
}

/// Enables `chart.facet(("category", 3))`
impl From<(&str, usize)> for FacetSpec {
    fn from((field, columns): (&str, usize)) -> Self {
        FacetSpec::wrap(field).with_columns(columns)
    }
}

/// Enables `chart.facet(("row_field", "col_field"))`
impl From<(&str, &str)> for FacetSpec {
    fn from((row_field, col_field): (&str, &str)) -> Self {
        FacetSpec::grid(row_field, col_field)
    }
}

// ============== Strategy Enums & Conversions ==============

/// Determines how axes are shared across panels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FacetStrategy {
    Fixed, // Shared scales across all panels
    Free,  // Completely independent scales
    FreeX, // Shared Y, Independent X
    FreeY, // Shared X, Independent Y
}

impl FacetStrategy {
    /// Determines whether the bottom (X) and left (Y) axes are shown for a panel.
    ///
    /// `is_last_in_column` is needed by wrap layouts whose final row may be
    /// incomplete. For example, with five panels and three columns, the third
    /// panel is the last panel in its column even though it belongs to the
    /// first visual row.
    pub(crate) const fn axis_visibility(
        self,
        row: usize,
        col: usize,
        total_rows: usize,
        // Whether this panel is the last actual panel in its column.
        is_last_in_column: bool,
    ) -> (bool, bool) {
        // Shared X axes belong on the bottom-most actual panel of each column;
        // shared Y axes belong on the left-most column.
        let is_bottom = is_last_in_column || row + 1 == total_rows;
        let is_left = col == 0;

        match self {
            Self::Fixed => (is_bottom, is_left),
            Self::Free => (true, true),
            Self::FreeX => (true, is_left),
            Self::FreeY => (is_bottom, true),
        }
    }
}

/// Enables ergonomically setting strategy via string literals (e.g. `"free_x"`).
impl From<&str> for FacetStrategy {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "free" => FacetStrategy::Free,
            "freex" | "free_x" => FacetStrategy::FreeX,
            "freey" | "free_y" => FacetStrategy::FreeY,
            "fixed" => FacetStrategy::Fixed,
            _ => FacetStrategy::Fixed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FacetSpec;
    use crate::coordinate::Rect;
    use crate::theme::Theme;

    #[test]
    fn facet_panels_keep_equal_plot_dimensions() {
        let container = Rect::new(0.0, 0.0, 1000.0, 800.0);
        let theme = Theme::default();

        let grid = FacetSpec::grid("row", "column").into_facet();
        let grid_panels = grid.compute_panels(
            &[
                vec!["r1".to_string(), "r2".to_string()],
                vec!["c1".to_string(), "c2".to_string()],
            ],
            &container,
            &theme,
        );
        let grid_size = (grid_panels[0].rect.width, grid_panels[0].rect.height);
        assert!(
            grid_panels
                .iter()
                .all(|panel| (panel.rect.width, panel.rect.height) == grid_size)
        );

        let wrap = FacetSpec::wrap("category").with_columns(2).into_facet();
        let wrap_panels = wrap.compute_panels(
            &[vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
            ]],
            &container,
            &theme,
        );
        let wrap_size = (wrap_panels[0].rect.width, wrap_panels[0].rect.height);
        assert!(
            wrap_panels
                .iter()
                .all(|panel| (panel.rect.width, panel.rect.height) == wrap_size)
        );
    }
}

// ============== Layout Structures ==============

/// Metadata for a single panel within a faceted grid.
#[derive(Debug, Clone)]
pub struct FacetPanelInfo {
    pub row: usize,
    pub col: usize,
    pub total_rows: usize,
    pub total_cols: usize,
    /// The display label for the panel (e.g., "Year: 2023").
    pub label: String,
    /// The row-level facet value for this cell.
    pub row_label: String,
    /// The column-level facet value for this cell.
    pub col_label: String,
    /// The facet filter: `(field_name, value)` pairs used to filter the
    /// underlying rows for this panel. Each layer must keep only the rows
    /// whose facet field(s) match these exact values.
    ///
    /// Empty means no filtering (i.e., a non-faceted, single-panel chart).
    pub facet_filter: Vec<(String, String)>,
    pub show_x_axis: bool,
    pub show_y_axis: bool,
}

/// A resolved facet panel containing its physical bounds.
#[derive(Clone)]
pub struct FacetPanel {
    /// The actual data plotting area (Inner Box). Excludes axes, ticks, and titles.
    pub rect: Rect,
    /// The area where the category label (strip) is drawn.
    pub header_rect: Rect,
    pub info: FacetPanelInfo,
}
