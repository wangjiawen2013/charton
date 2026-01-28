use crate::facets::{Facet, FacetCell, FacetInfo, FacetLayout, FacetStrategy};
use crate::coordinate::Rect;
use crate::theme::Theme;

/// FacetWrap partitions data by a single variable and wraps panels into a 2D grid.
pub struct FacetWrap {
    pub field: String,
    pub strategy: FacetStrategy,
    pub rows: Option<usize>,
    pub cols: Option<usize>,
}

impl Facet for FacetWrap {
    fn fields(&self) -> Vec<String> { vec![self.field.clone()] }
    fn strategy(&self) -> FacetStrategy { self.strategy }

    fn compute_layout(&self, factors: &[Vec<String>], container: &Rect, theme: &Theme) -> FacetLayout {
        let values = &factors[0]; // Wrap only uses the first variable
        let n = values.len();
        
        // Calculate grid dimensions
        let (rows, cols) = match (self.rows, self.cols) {
            (Some(r), Some(c)) => (r, c),
            (Some(r), None) => (r, (n as f64 / r as f64).ceil() as usize),
            (None, Some(c)) => ((n as f64 / c as f64).ceil() as usize, c),
            (None, None) => {
                let c = (n as f64).sqrt().ceil() as usize;
                let r = (n as f64 / c as f64).ceil() as usize;
                (r, c)
            }
        };

        let header_h = theme.facet_label_size * 1.5;
        let gap = theme.facet_spacing;

        let panel_w = (container.width - (cols - 1) as f64 * gap) / cols as f64;
        let panel_h = (container.height - (rows - 1) as f64 * gap - rows as f64 * header_h) / rows as f64;

        let mut cells = Vec::new();
        for (idx, val) in values.iter().enumerate() {
            let r = idx / cols;
            let c = idx % cols;

            let x = container.x + c as f64 * (panel_w + gap);
            let header_y = container.y + r as f64 * (panel_h + header_h + gap);
            let plot_y = header_y + header_h;

            cells.push(FacetCell {
                rect: Rect::new(x, plot_y, panel_w, panel_h),
                header_rect: Rect::new(x, header_y, panel_w, header_h),
                info: FacetInfo { row: r, col: c, total_rows: rows, total_cols: cols, label: val.clone() },
            });
        }
        FacetLayout { cells }
    }
}