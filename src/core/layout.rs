use super::context::PanelContext;
use super::guide::{GuideSpec, LegendPosition};
use super::utils::estimate_text_width;
use crate::theme::Theme;

/// Physical constraints calculated for axis areas.
#[derive(Default, Debug, Clone, Copy)]
pub struct AxisLayoutConstraints {
    pub bottom: f32, 
    pub left: f32,   
}

/// Margin reserved on each side of the plot for legend placement.
#[derive(Default, Debug, Clone, Copy)]
pub struct LegendLayoutConstraints {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

pub struct LayoutEngine;

impl LayoutEngine {
    /// Calculates legend margins using a greedy stacking algorithm.
    /// 
    /// The logic follows a "Flex-box" style approach:
    /// 1. **Vertical Stacking (Right/Left)**: Legends are stacked in a column. 
    ///    If a legend exceeds `initial_plot_h`, a new column is started to the side.
    /// 2. **Horizontal Stacking (Top/Bottom)**: Legends are laid out in a row.
    ///    If a legend exceeds `initial_plot_w`, a new row is started below/above.
    pub fn calculate_legend_constraints(
        specs: &[GuideSpec],
        position: LegendPosition,
        canvas_w: f32,
        canvas_h: f32,
        initial_plot_w: f32,
        initial_plot_h: f32,
        margin_gap: f32, // Space between plot panel and the whole legend block
        theme: &Theme,
    ) -> LegendLayoutConstraints {
        let mut constraints = LegendLayoutConstraints::default();
        if specs.is_empty() {
            return constraints;
        }

        let block_gap = theme.legend_block_gap;

        match position {
            LegendPosition::Right | LegendPosition::Left => {
                // For side-aligned legends, the height is constrained by the plot area.
                let max_h = initial_plot_h;
                
                let mut total_width = 0.0;
                let mut current_col_w = 0.0;
                let mut current_col_h = 0.0;

                for (i, spec) in specs.iter().enumerate() {
                    let size = spec.estimate_size(theme, max_h);

                    // If this block overflows the current column height, move to the next column
                    // We only wrap if we already have at least one item in the current column.
                    if current_col_h + size.height > max_h && current_col_h > 0.0 {
                        total_width += current_col_w + block_gap;
                        current_col_w = size.width;
                        current_col_h = size.height;
                    } else {
                        // Expand column width if this block is wider than previous ones in the same column
                        current_col_w = f32::max(current_col_w, size.width);
                        current_col_h += size.height;
                        
                        // Add gap between blocks, but not after the last one in the column
                        if i < specs.len() - 1 {
                            current_col_h += block_gap;
                        }
                    }
                }
                total_width += current_col_w;

                // Safety cap: Prevent legends from consuming too much horizontal space.
                // We ensure the plot panel has a "Defense Floor".
                let min_panel_w = f32::max(theme.min_panel_size, canvas_w * theme.panel_defense_ratio);
                let max_allowed_legend_w = (canvas_w - min_panel_w - theme.axis_reserve_buffer).max(0.0);
                
                let final_w = f32::min(total_width, max_allowed_legend_w);
                let reserve = if final_w > 0.0 { final_w + margin_gap } else { 0.0 };

                if position == LegendPosition::Right { constraints.right = reserve; }
                else { constraints.left = reserve; }
            }

            LegendPosition::Top | LegendPosition::Bottom => {
                // For top/bottom legends, the width is constrained by the plot area.
                let max_w = initial_plot_w;
                let mut total_height = 0.0;
                let mut current_row_h = 0.0;
                let mut current_row_w = 0.0;

                for (i, spec) in specs.iter().enumerate() {
                    // Capping the height of individual legend items for horizontal layout
                    // to prevent "squashing" the plot panel vertically.
                    let size = spec.estimate_size(theme, canvas_h * 0.25);

                    if current_row_w + size.width > max_w && current_row_w > 0.0 {
                        total_height += current_row_h + block_gap;
                        current_row_h = size.height;
                        current_row_w = size.width;
                    } else {
                        current_row_h = f32::max(current_row_h, size.height);
                        current_row_w += size.width;
                        
                        if i < specs.len() - 1 {
                            current_row_w += block_gap;
                        }
                    }
                }
                total_height += current_row_h;

                let min_panel_h = f32::max(theme.min_panel_size, canvas_h * theme.panel_defense_ratio);
                let max_allowed_legend_h = (canvas_h - min_panel_h - theme.axis_reserve_buffer).max(0.0);
                
                let final_h = f32::min(total_height, max_allowed_legend_h);
                let reserve = if final_h > 0.0 { final_h + margin_gap } else { 0.0 };

                if position == LegendPosition::Top { constraints.top = reserve; }
                else { constraints.bottom = reserve; }
            },
            LegendPosition::None => { return constraints; }
        }
        constraints
    }

    /// Calculates axis space using scale ticks and text measurement.
    /// Labels are now retrieved directly from the coordinate system.
    pub fn calculate_axis_constraints(
        ctx: &PanelContext,
        theme: &Theme,
    ) -> AxisLayoutConstraints {
        let mut constraints = AxisLayoutConstraints::default();
        let coord = ctx.coord.clone();
        let is_flipped = coord.is_flipped();

        // 1. Resolve Bottom Axis:
        // Uses X-scale by default; uses Y-scale if the coordinate system is flipped.
        let (b_scale, b_angle, b_title, b_pad) = if is_flipped {
            (coord.get_y_scale(), theme.y_tick_label_angle, coord.get_y_label(), theme.label_padding)
        } else {
            (coord.get_x_scale(), theme.x_tick_label_angle, coord.get_x_label(), theme.label_padding)
        };
        constraints.bottom = Self::estimate_axis_dimension(b_scale, b_angle, b_title, b_pad, theme, true);

        // 2. Resolve Left Axis:
        // Uses Y-axis by default, or X-axis if flipped.
        let (l_scale, l_angle, l_title, l_pad) = if is_flipped {
            (coord.get_x_scale(), theme.x_tick_label_angle, coord.get_x_label(), theme.label_padding)
        } else {
            (coord.get_y_scale(), theme.y_tick_label_angle, coord.get_y_label(), theme.label_padding)
        };
        constraints.left = Self::estimate_axis_dimension(l_scale, l_angle, l_title, l_pad, theme, false);

        constraints
    }

    /// Estimates the 'depth' (width for Y, height for X) required for an axis.
    /// It considers tick marks, rotated labels, and axis titles.
    fn estimate_axis_dimension(
        scale: &dyn crate::scale::ScaleTrait,
        angle_deg: f32,
        title: &str,
        label_padding: f32,
        theme: &Theme,
        is_horizontal_axis: bool,
    ) -> f32 {
        let tick_line_len = 6.0;
        let edge_buffer = 10.0;
        let angle_rad = angle_deg.to_radians();
        
        // Use the scale's own tick generation logic for measurement
        let ticks = scale.ticks(8);

        // Compute the "footprint" of labels. 
        // If rotated 90 degrees, a wide label becomes a deep axis.
        let max_label_footprint = ticks.iter()
            .map(|t| {
                let w = estimate_text_width(&t.label, theme.tick_label_size);
                let h = theme.tick_label_size;
                if is_horizontal_axis {
                    // Vertical footprint for Bottom Axis: w*sin + h*cos
                    w.abs() * angle_rad.sin().abs() + h * angle_rad.cos().abs()
                } else {
                    // Horizontal footprint for Left Axis: w*cos + h*sin
                    w.abs() * angle_rad.cos().abs() + h * angle_rad.sin().abs()
                }
            })
            .fold(0.0, f32::max);

        let title_area = if title.is_empty() { 
            0.0 
        } else { 
            theme.label_size + label_padding + 5.0 
        };

        tick_line_len + theme.tick_label_padding + max_label_footprint + title_area + edge_buffer
    }
}