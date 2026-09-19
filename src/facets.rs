//! Faceting module for creating multi-panel visualizations.
//!
//! This module provides the core infrastructure for splitting charts into
//! multiple panels based on data fields.
//!
//! # The two invariants
//!
//! A faceted chart is a single visual object, so its panels must all be the same
//! size and must line up on shared row/column lines. Both invariants are met by
//! laying the grid out on **tracks** rather than on equally divided cells:
//! [`FacetGridGeometry`] owns one width per column and one height per row (axes
//! and headers included), and every panel is placed at the intersection of a
//! panel row and a panel column.
//!
//! # Who owns the axis space
//!
//! A single chart reserves its axes on the outer border (see `layout`). A facet
//! grid instead reserves them inside, on the specific columns and rows that draw
//! them, using the extents handed down as [`FacetMetrics`]. The surrounding
//! layout therefore leaves the panel rect whole for a faceted chart.
//!
//! For the full picture, see the *Multi-View* chapter in the book.

mod facet_grid;
mod facet_wrap;

use crate::coordinate::Rect;
use crate::theme::Theme;
pub use facet_grid::FacetGridImpl;
pub use facet_wrap::FacetWrapImpl;

/// Axis space the surrounding layout has measured for a facet grid.
///
/// A single chart reserves its axis space globally in `LayoutEngine`. A facet
/// grid cannot do that: the space an axis needs depends on which panel it belongs
/// to, and it must only be reserved on the columns/rows that actually draw one.
/// The measured extents are therefore handed down here and expanded into tracks
/// by [`FacetGridGeometry`].
///
/// This type answers *how big* an axis is, never *where* it goes: it carries no
/// positions. Deciding placement is the facet implementation's job, so the two
/// questions stay in one place each and cannot drift apart. The numbers are the
/// same ones the `Space Manager` measures for a single chart, which is why a
/// faceted axis is exactly as large as its non-faceted counterpart.
///
/// The names are physical, not logical: `axis_left` is the width of the **y**
/// axis (the one drawn on the left), and `axis_bottom` is the height of the
/// **x** axis (the one drawn on the bottom), regardless of `coord_flip`.
#[derive(Debug, Clone, Copy, Default)]
pub struct FacetMetrics {
    /// Width reserved to the left of a panel that draws a y axis.
    pub axis_left: f64,
    /// Height reserved below a panel that draws an x axis.
    pub axis_bottom: f64,
}

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
    /// Implementations must satisfy the two invariants a facet is judged on:
    /// every returned panel has the **same size**, and panels sharing a row or a
    /// column share its start coordinate. Both follow naturally from laying the
    /// grid out on tracks; see [`FacetGridGeometry`].
    ///
    /// # Arguments
    /// * `factors` - The unique values from the data fields, in the same order
    ///   as returned by `fields()`.
    /// * `container` - The total area available for all facets. It already
    ///   excludes margins and guides, and it includes the axis tracks: a faceted
    ///   chart does not reserve axis space on the outer border.
    /// * `metrics` - The measured axis extents to turn into grid tracks.
    /// * `theme` - Theme settings for spacing and label sizes.
    ///
    /// # Returns
    /// A `Vec<FacetPanel>` ordered row-major. Each panel contains its plot
    /// rectangle, header rectangle, and facet filter for data subsetting.
    fn compute_panels(
        &self,
        factors: &[Vec<String>],
        container: &Rect,
        metrics: &FacetMetrics,
        theme: &Theme,
    ) -> Vec<FacetPanel>;
}

/// Resolved geometry of a regular facet grid.
///
/// The grid is a set of **tracks**, not equally divided cells:
///
/// ```text
///   column:  [axis-left?] [   panel   ] spacing [axis-left?] [   panel   ] ...
///   row:     [  header  ] [   panel   ] [axis-bottom?] spacing ...
/// ```
///
/// A **track** is one row or one column of the grid, with its own size. A panel
/// sits at the intersection of a *panel column* and a *panel row*; axes and
/// headers own tracks of their own. This is exactly the HTML-table / CSS-grid
/// model, and it gives the two properties a facet needs for free:
///
/// * panels are all the same size, because there is a single `plot_w` for every
///   panel column and a single `plot_h` for every panel row;
/// * panels are aligned, because every panel column starts at the same offset in
///   every row (and likewise for rows).
///
/// A track is only reserved where an axis is actually drawn, so a `Fixed` grid
/// keeps the y axis on its first column and the x axis on its last row, while a
/// `Free` grid reserves them on every column/row. The only space between two
/// neighbouring panels is then `theme.facet_spacing`, instead of that plus a
/// per-cell axis reservation.
///
/// The axis extents themselves come from [`FacetMetrics`]; the header height and
/// the spacing come from the theme.
pub(crate) struct FacetGridGeometry {
    /// Width shared by every panel (a panel column's content track).
    plot_w: f64,
    /// Height shared by every panel (a panel row's content track).
    plot_h: f64,
    /// Height of the facet strip drawn above every panel.
    header_h: f64,
    /// Left edge of every column's panel, i.e. *after* that column's axis track.
    col_x: Vec<f64>,
    /// Top edge of every row's header strip.
    header_y: Vec<f64>,
}

/// Smallest panel edge the grid will produce. A canvas too small to honour all
/// tracks can only overflow; this keeps the degenerate case finite.
const MIN_FACET_PANEL: f64 = 40.0;

impl FacetGridGeometry {
    /// Resolves the grid.
    ///
    /// `has_left_axis[c]` / `has_bottom_axis[r]` say whether the column/row needs
    /// an axis track. They are derived by the caller from `axis_visibility`, because
    /// only the caller knows which cells exist (wrap's last row may be partial).
    ///
    /// The work happens in two steps: first solve the one panel size every cell
    /// shares, then accumulate the track offsets. Both steps account for the same
    /// set of tracks, so the panels always add up to the container exactly.
    pub(crate) fn new(
        rows: usize,
        cols: usize,
        container: &Rect,
        metrics: &FacetMetrics,
        theme: &Theme,
        has_left_axis: &[bool],
        has_bottom_axis: &[bool],
    ) -> Self {
        let spacing = theme.facet_spacing;
        let header_h = theme.facet_label_size * 1.5 + theme.facet_strip_padding * 2.0;

        // How many axis tracks of each kind are actually present. `Fixed` says 1
        // and 1; `Free` says `cols` and `rows`.
        let left_tracks = has_left_axis.iter().filter(|flag| **flag).count() as f64;
        let bottom_tracks = has_bottom_axis.iter().filter(|flag| **flag).count() as f64;

        // --- Step 1: solve the panel size ------------------------------------
        //
        // The horizontal axis is laid out as
        //   [y-axis] panel (gap panel)*
        // so it loses `left_tracks` axis tracks and one gap between each pair of
        // columns. The vertical axis is laid out as
        //   (header panel [x-axis]) (gap header panel [x-axis])*
        // so it loses one header *per row* plus `bottom_tracks` axis tracks plus
        // the gaps between rows. What is left is split evenly, because all panels
        // share a single width and a single height.
        let panel_area_w = container.width
            - left_tracks * metrics.axis_left
            - cols.saturating_sub(1) as f64 * spacing;
        let panel_area_h = container.height
            - bottom_tracks * metrics.axis_bottom
            - rows as f64 * header_h
            - rows.saturating_sub(1) as f64 * spacing;

        // The floor only matters on a canvas too small for the requested grid: it
        // keeps the geometry finite and lets the grid overflow instead of
        // collapsing to a negative size.
        let plot_w = (panel_area_w / cols.max(1) as f64).max(MIN_FACET_PANEL);
        let plot_h = (panel_area_h / rows.max(1) as f64).max(MIN_FACET_PANEL);

        // --- Step 2: accumulate track offsets --------------------------------
        //
        // Column `c`'s panel starts after every y-axis track up to and including
        // column `c`, plus the `c` panels and `c` gaps that come before it:
        //
        //   col_x[c] = container.x + Σ_{i<=c} [axis(i)] + c * (plot_w + spacing)
        //
        // `consumed` is the running Σ above; it is advanced *before* the panel is
        // placed, because the axis track sits to the left of its own panel.
        let mut col_x = Vec::with_capacity(cols);
        let mut consumed = 0.0;
        for c in 0..cols {
            if has_left_axis.get(c).copied().unwrap_or(false) {
                consumed += metrics.axis_left;
            }
            col_x.push(container.x + consumed + c as f64 * (plot_w + spacing));
        }

        // Row `r`'s header starts after the x-axis tracks of the rows *above* r,
        // plus the `r` groups of (header + panel) and the `r` gaps before it:
        //
        //   header_y[r] = container.y + Σ_{j<r} [axis(j)] + r * (header_h + plot_h + spacing)
        //
        // `consumed` is the running Σ above; unlike the x case it is advanced
        // *after* the header, because an x-axis track sits below its own row.
        let mut header_y = Vec::with_capacity(rows);
        let mut consumed = 0.0;
        for r in 0..rows {
            header_y.push(container.y + consumed + r as f64 * (header_h + plot_h + spacing));
            if has_bottom_axis.get(r).copied().unwrap_or(false) {
                consumed += metrics.axis_bottom;
            }
        }

        Self {
            plot_w,
            plot_h,
            header_h,
            col_x,
            header_y,
        }
    }

    /// The plotting rectangle of one cell: the shared panel size, placed at the
    /// column's x and just below the row's header. Axis tracks are deliberately
    /// *not* part of this rect -- they belong to the grid, not to the panel.
    pub(crate) fn panel_rect(&self, row: usize, col: usize) -> Rect {
        Rect::new(
            self.col_x[col],
            self.header_y[row] + self.header_h,
            self.plot_w,
            self.plot_h,
        )
    }

    /// The header (facet strip) rectangle above one cell's panel. It shares the
    /// panel's x and width so the strip label stays centred over the plot.
    pub(crate) fn header_rect(&self, row: usize, col: usize) -> Rect {
        Rect::new(
            self.col_x[col],
            self.header_y[row],
            self.plot_w,
            self.header_h,
        )
    }
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
    use super::{FacetMetrics, FacetSpec};
    use crate::coordinate::Rect;
    use crate::theme::Theme;

    fn metrics() -> FacetMetrics {
        FacetMetrics {
            axis_left: 50.0,
            axis_bottom: 40.0,
        }
    }

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
            &metrics(),
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
            &metrics(),
            &theme,
        );
        let wrap_size = (wrap_panels[0].rect.width, wrap_panels[0].rect.height);
        assert!(
            wrap_panels
                .iter()
                .all(|panel| (panel.rect.width, panel.rect.height) == wrap_size)
        );
    }

    /// Two neighbouring panels must be separated by exactly `facet_spacing`.
    /// The axis tracks are reserved on the grid's own edges, not between cells.
    #[test]
    fn neighbouring_panels_are_separated_by_facet_spacing_only() {
        let container = Rect::new(10.0, 20.0, 1000.0, 800.0);
        let theme = Theme::default();
        let m = metrics();

        // A full 2x2 grid with shared axes: y axis on column 0 only, x axis on
        // the last row only.
        let grid = FacetSpec::grid("row", "column").into_facet();
        let panels = grid.compute_panels(
            &[
                vec!["r1".to_string(), "r2".to_string()],
                vec!["c1".to_string(), "c2".to_string()],
            ],
            &container,
            &m,
            &theme,
        );

        // Row-major: 0=(0,0) 1=(0,1) 2=(1,0) 3=(1,1).
        let horizontal_gap = panels[1].rect.x - (panels[0].rect.x + panels[0].rect.width);
        assert!((horizontal_gap - theme.facet_spacing).abs() < 1e-9);

        let vertical_gap = panels[2].rect.y - (panels[0].rect.y + panels[0].rect.height);
        // The lower row still has to fit its own header strip between the panels.
        let header_h = theme.facet_label_size * 1.5 + theme.facet_strip_padding * 2.0;
        assert!((vertical_gap - (theme.facet_spacing + header_h)).abs() < 1e-9);

        // The shared y axis sits in a track to the left of column 0, so the
        // first panel starts one axis width inside the container.
        assert!((panels[0].rect.x - (container.x + m.axis_left)).abs() < 1e-9);
    }

    /// A free grid draws an axis around every panel, so every column/row keeps
    /// its track and the panels stay equal nonetheless.
    #[test]
    fn free_grid_reserves_axis_tracks_everywhere() {
        let container = Rect::new(0.0, 0.0, 1000.0, 800.0);
        let theme = Theme::default();
        let m = metrics();

        let grid = FacetSpec::grid("row", "column")
            .with_strategy("free")
            .into_facet();
        let panels = grid.compute_panels(
            &[
                vec!["r1".to_string(), "r2".to_string()],
                vec!["c1".to_string(), "c2".to_string()],
            ],
            &container,
            &m,
            &theme,
        );

        // Every column starts with its own y-axis track.
        assert!(
            (panels[1].rect.x
                - (panels[0].rect.x + panels[0].rect.width + theme.facet_spacing + m.axis_left))
                .abs()
                < 1e-9
        );
        // Every panel is the same size.
        assert!(panels.iter().all(|p| p.rect.width == panels[0].rect.width));
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
