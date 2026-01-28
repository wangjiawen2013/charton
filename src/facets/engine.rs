use crate::coordinate::Rect;

/// Determines how axes are shared across panels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FacetStrategy {
    Fixed,  // Shared scales
    Free,   // Independent scales
    FreeX,  // Shared Y, Independent X
    FreeY,  // Shared X, Independent Y
}

/// Metadata for a single panel.
#[derive(Debug, Clone)]
pub struct FacetInfo {
    pub row: usize,
    pub col: usize,
    pub total_rows: usize,
    pub total_cols: usize,
    /// The display label (e.g., "Year: 2023").
    pub label: String,
}

/// A resolved facet cell containing its physical bounds.
pub struct FacetCell {
    /// The actual data plotting area (Inner Box). Excludes axes, ticks, and titles.
    pub rect: Rect,
    /// The area where the category label (strip) is drawn.
    pub header_rect: Rect,
    pub info: FacetInfo,
}

/// The physical layout result of any faceting operation.
/// This is what the Renderer consumes.
pub struct FacetLayout {
    pub cells: Vec<FacetCell>,
}