use crate::facets::{Facet, FacetLayout, FacetCell, FacetInfo, FacetStrategy};
use crate::coordinate::Rect;
use crate::theme::Theme;

/// FacetGrid partitions data by two variables into a strict Row x Column matrix.
pub struct FacetGrid {
    pub row_field: String,
    pub col_field: String,
    pub strategy: FacetStrategy,
}

impl Facet for FacetGrid {
    fn fields(&self) -> Vec<String> { vec![self.row_field.clone(), self.col_field.clone()] }
    fn strategy(&self) -> FacetStrategy { self.strategy }

    fn compute_layout(&self, factors: &[Vec<String>], container: &Rect, theme: &Theme) -> FacetLayout {
        let rows_vals = &factors[0];
        let cols_vals = &factors[1];
        
        let n_rows = rows_vals.len();
        let n_cols = cols_vals.len();

        let header_h = theme.facet_label_size * 1.5;
        let gap = theme.facet_spacing;

        let panel_w = (container.width - (n_cols - 1) as f64 * gap) / n_cols as f64;
        let panel_h = (container.height - (n_rows - 1) as f64 * gap - n_rows as f64 * header_h) / n_rows as f64;

        let mut cells = Vec::new();
        for (r_idx, r_val) in rows_vals.iter().enumerate() {
            for (c_idx, c_val) in cols_vals.iter().enumerate() {
                let x = container.x + c_idx as f64 * (panel_w + gap);
                let header_y = container.y + r_idx as f64 * (panel_h + header_h + gap);
                
                cells.push(FacetCell {
                    rect: Rect::new(x, header_y + header_h, panel_w, panel_h),
                    header_rect: Rect::new(x, header_y, panel_w, header_h),
                    info: FacetInfo {
                        row: r_idx,
                        col: c_idx,
                        total_rows: n_rows,
                        total_cols: n_cols,
                        label: format!("{} | {}", r_val, c_val), // Combined label for now
                    },
                });
            }
        }
        FacetLayout { cells }
    }
}