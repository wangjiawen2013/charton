use super::context::SharedRenderingContext;
use super::legend::{LegendSpec, LegendPosition};
use super::utils::estimate_text_width;
use crate::theme::Theme;

/// Physical constraints calculated for axis areas.
/// These represent the pixel depth required to draw the axis ticks and labels.
#[derive(Default, Debug, Clone, Copy)]
pub struct AxisLayoutConstraints {
    pub bottom: f64, // Space reserved for the X-axis (or Y if flipped)
    pub left: f64,   // Space reserved for the Y-axis (or X if flipped)
}

/// Margin reserved on each side of the plot for legend placement.
/// This acts as a structural buffer calculated during the layout phase.
#[derive(Default, Debug, Clone, Copy)]
pub struct LegendLayoutConstraints {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

/// The LayoutEngine is responsible for the geometric orchestration of the chart.
/// It resolves the conflicting space requirements between the Data Panel, the Axes, and the Legends.
pub struct LayoutEngine;

impl LayoutEngine {
    /// Calculates legend margins using a greedy stacking algorithm with a defensive floor.
    /// 
    /// This function handles two primary flow directions:
    /// 1. **Vertical Stacking (Left/Right)**: Blocks flow top-to-bottom and wrap into new columns.
    /// 2. **Horizontal Stacking (Top/Bottom)**: Blocks flow left-to-right and wrap into new rows.
    pub fn calculate_legend_constraints(
        specs: &[LegendSpec],
        position: LegendPosition,
        canvas_w: f64,
        canvas_h: f64,
        initial_plot_w: f64, // Theoretical width available for the plot
        initial_plot_h: f64, // Theoretical height available for the plot
        margin_gap: f64,     // Buffer space between the plot edge and legend blocks
        theme: &Theme,
    ) -> LegendLayoutConstraints {
        let mut constraints = LegendLayoutConstraints::default();
        
        // Short-circuit if no legends are required
        if position == LegendPosition::None || specs.is_empty() {
            return constraints;
        }

        let block_gap = theme.legend_block_gap;

        match position {
            // --- SIDE LAYOUTS (Vertical Flow: Blocks stack vertically, then wrap to new columns) ---
            LegendPosition::Right | LegendPosition::Left => {
                let max_h = initial_plot_h;
                let mut total_width = 0.0;
                let mut current_col_w = 0.0;
                let mut current_col_h = 0.0;

                for spec in specs {
                    // Estimate the size of the individual legend block (e.g., "color of gear")
                    println!("initial_plot_h: {}, initial_plot_w: {}", initial_plot_h, initial_plot_w);
                    let size = spec.estimate_size(theme, max_h);
                    println!("{:?}", size);
                    // WRAPPING LOGIC: Check if adding this block exceeds the vertical limit of the plot
                    if current_col_h + size.height > max_h && current_col_h > 0.0 {
                        // Current column is full. Commit current column width to total.
                        total_width += current_col_w + block_gap;
                        // Reset for a new column starting with this block
                        current_col_w = size.width;
                        current_col_h = size.height + block_gap;
                    } else {
                        // Block fits in current column. Update max width and accumulate height.
                        current_col_w = f64::max(current_col_w, size.width);
                        current_col_h += size.height + block_gap;
                    }
                }
                // Add the final column's width
                total_width += current_col_w;

                // DEFENSE MECHANISM: Ensure legends don't consume too much horizontal space.
                // We calculate a minimum width for the chart panel based on theme preferences.
                let min_panel_w = f64::max(theme.min_panel_size, canvas_w * theme.panel_defense_ratio);
                let max_allowed_legend_w = (canvas_w - min_panel_w - theme.axis_reserve_buffer).max(0.0);
                
                let final_w = f64::min(total_width, max_allowed_legend_w);
                let reserve = final_w + margin_gap;

                if position == LegendPosition::Right {
                    constraints.right = reserve;
                } else {
                    constraints.left = reserve;
                }
            }

            // --- TOP/BOTTOM LAYOUTS (Horizontal Flow: Blocks stack horizontally, then wrap to new rows) ---
            LegendPosition::Top | LegendPosition::Bottom => {
                let max_w = initial_plot_w;
                let mut total_height = 0.0;
                let mut current_row_h = 0.0;
                let mut current_row_w = 0.0;

                for spec in specs {
                    let size = spec.estimate_size(theme, max_w);

                    // WRAPPING LOGIC: Check if adding this block exceeds the horizontal limit
                    if current_row_w + size.width > max_w && current_row_w > 0.0 {
                        // Current row is full. Commit current row height to total.
                        total_height += current_row_h + block_gap;
                        // Reset for a new row starting with this block
                        current_row_h = size.height;
                        current_row_w = size.width + block_gap;
                    } else {
                        // Block fits in current row. Update max height and accumulate width.
                        current_row_h = f64::max(current_row_h, size.height);
                        current_row_w += size.width + block_gap;
                    }
                }
                // Add the final row's height
                total_height += current_row_h;

                // DEFENSE MECHANISM: Ensure legends don't consume too much vertical space.
                let min_panel_h = f64::max(100.0, canvas_h * 0.2);
                let max_allowed_legend_h = (canvas_h - min_panel_h - 60.0).max(0.0);
                
                let final_h = f64::min(total_height, max_allowed_legend_h);
                let reserve = final_h + margin_gap;

                if position == LegendPosition::Top {
                    constraints.top = reserve;
                } else {
                    constraints.bottom = reserve;
                }
            }
            _ => {}
        }
        constraints
    }

    /// Calculates the space required for axis ticks, labels, and titles.
    /// This is typically called after legend constraints are known to finalize the panel size.
    pub fn calculate_axis_constraints(
        ctx: &SharedRenderingContext,
        theme: &Theme,
        x_label: &str,
        y_label: &str,
    ) -> AxisLayoutConstraints {
        let mut constraints = AxisLayoutConstraints::default();
        let coord = ctx.coord;
        let is_flipped = coord.is_flipped();

        // 1. Calculate Bottom Axis (Horizontal depth)
        // Maps to Y-scale data if the coordinates are flipped.
        let (b_scale, b_angle, b_title, b_pad) = if is_flipped {
            (coord.get_y_scale(), theme.y_tick_label_angle, y_label, theme.label_padding)
        } else {
            (coord.get_x_scale(), theme.x_tick_label_angle, x_label, theme.label_padding)
        };
        constraints.bottom = Self::estimate_axis_dimension(b_scale, b_angle, b_title, b_pad, theme, true);

        // 2. Calculate Left Axis (Vertical depth)
        // Maps to X-scale data if the coordinates are flipped.
        let (l_scale, l_angle, l_title, l_pad) = if is_flipped {
            (coord.get_x_scale(), theme.x_tick_label_angle, x_label, theme.label_padding)
        } else {
            (coord.get_y_scale(), theme.y_tick_label_angle, y_label, theme.label_padding)
        };
        constraints.left = Self::estimate_axis_dimension(l_scale, l_angle, l_title, l_pad, theme, false);

        constraints
    }

    /// Measures the 'depth' (width for vertical axis, height for horizontal axis) 
    /// required by an axis.
    fn estimate_axis_dimension(
        scale: &dyn crate::scale::ScaleTrait,
        angle_deg: f64,
        title: &str,
        label_padding: f64,
        theme: &Theme,
        is_horizontal_axis: bool,
    ) -> f64 {
        let tick_line_len = 6.0;
        let edge_buffer = 10.0;
        let angle_rad = angle_deg.to_radians();
        let ticks = scale.ticks(8);

        // Calculate the maximum label projection after rotation.
        // For a horizontal axis, rotation increases the vertical footprint (Y projection).
        // For a vertical axis, rotation increases the horizontal footprint (X projection).
        let max_label_footprint = ticks.iter()
            .map(|t| {
                let w = estimate_text_width(&t.label, theme.tick_label_size);
                let h = theme.tick_label_size;
                if is_horizontal_axis {
                    w.abs() * angle_rad.sin().abs() + h * angle_rad.cos().abs()
                } else {
                    w.abs() * angle_rad.cos().abs() + h * angle_rad.sin().abs()
                }
            })
            .fold(0.0, f64::max);

        // Calculate space for the axis title if it exists.
        let title_area = if title.is_empty() { 
            0.0 
        } else { 
            theme.label_size + label_padding + 5.0 
        };

        tick_line_len + theme.tick_label_padding + max_label_footprint + title_area + edge_buffer
    }
}