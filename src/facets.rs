pub mod engine;
pub mod facet_grid;
pub mod facet_wrap;

pub use engine::{FacetCell, FacetInfo, FacetLayout, FacetStrategy};
pub use facet_grid::FacetGrid;
pub use facet_wrap::FacetWrap;

/// The core trait that all Faceting methods must implement.
/// This allows the LayoutEngine to treat Wrap and Grid polymorphically.
pub trait Facet {
    /// Returns the data column(s) required for faceting.
    fn fields(&self) -> Vec<String>;

    /// Returns the scale resolution strategy (Fixed vs Free).
    fn strategy(&self) -> FacetStrategy;

    /// Computes the physical grid layout.
    fn compute_layout(
        &self,
        factors: &[Vec<String>], // Supports multiple variables for Grid
        container: &crate::coordinate::Rect,
        theme: &crate::theme::Theme,
    ) -> FacetLayout;
}
