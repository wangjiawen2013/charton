use crate::coordinate::Rect;
use crate::facets::{Facet, FacetLayout, FacetPanel, FacetPanelInfo, FacetStrategy};
use crate::theme::Theme;

/// FacetGrid partitions data by two variables into a strict Row x Column matrix.
pub struct FacetGrid {
    pub row_field: String,
    pub col_field: String,
    pub strategy: FacetStrategy,
}

impl Facet for FacetGrid {
    fn fields(&self) -> Vec<String> {
        vec![self.row_field.clone(), self.col_field.clone()]
    }
    fn strategy(&self) -> FacetStrategy {
        self.strategy
    }

    fn compute_layout(
        &self,
        factors: &[Vec<String>],
        container: &Rect,
        theme: &Theme,
    ) -> FacetLayout {
        let rows_vals = &factors[0];
        let cols_vals = &factors[1];

        let n_rows = rows_vals.len();
        let n_cols = cols_vals.len();

        let header_h = theme.facet_label_size * 1.5 + theme.facet_strip_padding * 2.0;
        let gap = theme.facet_spacing;

        let panel_w = (container.width - (n_cols - 1) as f64 * gap) / n_cols as f64;
        let panel_h = (container.height - (n_rows - 1) as f64 * gap) / n_rows as f64;
        let left_pad = (theme.label_size + theme.tick_label_size + 12.0).max(24.0);
        let right_pad = 12.0;
        let top_pad = 8.0;
        let bottom_pad = (theme.label_size + theme.tick_label_size + 12.0).max(24.0);
        let plot_w = (panel_w - left_pad - right_pad).max(40.0);
        let plot_h = (panel_h - header_h - top_pad - bottom_pad).max(40.0);

        let mut cells = Vec::new();
        for (r_idx, r_val) in rows_vals.iter().enumerate() {
            for (c_idx, c_val) in cols_vals.iter().enumerate() {
                let x = container.x + c_idx as f64 * (panel_w + gap);
                let header_y = container.y + r_idx as f64 * (panel_h + gap);

                cells.push(FacetPanel {
                    rect: Rect::new(x + left_pad, header_y + header_h + top_pad, plot_w, plot_h),
                    header_rect: Rect::new(x, header_y, panel_w, header_h),
                    info: FacetPanelInfo {
                        row: r_idx,
                        col: c_idx,
                        total_rows: n_rows,
                        total_cols: n_cols,
                        label: format!("{} | {}", r_val, c_val),
                        row_label: r_val.clone(),
                        col_label: c_val.clone(),
                        facet_values: vec![r_val.clone(), c_val.clone()],
                    },
                });
            }
        }
        FacetLayout { cells }
    }
}
