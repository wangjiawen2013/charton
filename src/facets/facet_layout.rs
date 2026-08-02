use crate::coordinate::Rect;

/// Determines how axes are shared across panels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FacetStrategy {
    Fixed, // Shared scales
    Free,  // Independent scales
    FreeX, // Shared Y, Independent X
    FreeY, // Shared X, Independent Y
}

/// Metadata for a single panel.
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
    /// The facet values used to filter the underlying rows for this panel.
    pub facet_values: Vec<String>,
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

/// The physical layout result of any faceting operation.
/// This is what the Renderer consumes.
#[derive(Default, Clone)]
pub struct FacetLayout {
    pub cells: Vec<FacetPanel>,
}
