use super::context::SharedRenderingContext;
use super::legend::{LegendSpec, LegendPosition};
use super::utils::estimate_text_width;
use crate::theme::Theme;

/// Physical constraints calculated for axis areas.
/// These represent the pixel-width/height required to draw the axis ticks and labels.
#[derive(Default, Debug, Clone, Copy)]
pub struct AxisLayoutConstraints {
    pub bottom: f64, 
    pub left: f64,   
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
/// it resolves the conflicting space requirements between the Data Panel, the Axes, and the Legends.
pub struct LayoutEngine;

impl LayoutEngine {
    /// Calculates legend margins using a greedy stacking algorithm with a defensive floor.
    /// 
    /// Logic Flow:
    /// 1. Micro-layout: Each LegendSpec estimates its size given the `initial_plot_h` (Y-axis length).
    /// 2. Macro-layout: Multiple legend blocks are stacked vertically. If a block overflows 
    ///    the vertical space, it wraps to a new column on the side.
    /// 3. Defense: If the total width of all legend columns exceeds the available space,
    ///    it is capped to ensure the Plot Panel maintains a minimum size of 100px or 20% width.
    pub fn calculate_legend_constraints(
        specs: &[LegendSpec],
        position: LegendPosition,
        canvas_w: f64,
        canvas_h: f64,
        initial_plot_w: f64, // The theoretical plot width based solely on chart margins
        initial_plot_h: f64, // The theoretical plot height based solely on chart margins
        margin_gap: f64,     // The buffer space between the plot and the legend block
        theme: &Theme,
    ) -> LegendLayoutConstraints {
        let mut constraints = LegendLayoutConstraints::default();
        if position == LegendPosition::None || specs.is_empty() {
            return constraints;
        }

        let block_gap = theme.legend_block_gap; // Vertical/Horizontal gap between different legend blocks

        match position {
            LegendPosition::Right | LegendPosition::Left => {
                // The vertical limit for the legend is the height of the plot area
                let max_h = initial_plot_h;
                
                let mut total_width = 0.0;
                let mut current_col_w = 0.0;
                let mut current_col_h = 0.0;

                for spec in specs {
                    // Step 1: Individual block estimation (internal wrapping)
                    let size = spec.estimate_size(theme, max_h);

                    // Step 2: Column stacking logic
                    // If current block doesn't fit in the current column height, wrap to a new column.
                    if current_col_h + size.height > max_h && current_col_h > 0.0 {
                        total_width += current_col_w + block_gap;
                        current_col_w = size.width;
                        current_col_h = size.height;
                    } else {
                        // Keep track of the widest block in this column
                        current_col_w = f64::max(current_col_w, size.width);
                        current_col_h += size.height + block_gap;
                    }
                }
                total_width += current_col_w;

                // Step 3: Defense Mechanism using Theme properties
                let min_panel_w = f64::max(theme.min_panel_size, canvas_w * theme.panel_defense_ratio);
                let max_allowed_legend_w = (canvas_w - min_panel_w - theme.axis_reserve_buffer).max(0.0);
                
                let final_w = f64::min(total_width, max_allowed_legend_w);
                let reserve = final_w + margin_gap;

                if position == LegendPosition::Right { constraints.right = reserve; }
                else { constraints.left = reserve; }
            }

            LegendPosition::Top | LegendPosition::Bottom => {
                // Horizontal wrapping: legends stack left-to-right, wrapping to new rows.
                let max_w = initial_plot_w;
                let mut total_height = 0.0;
                let mut current_row_h = 0.0;
                let mut current_row_w = 0.0;

                for spec in specs {
                    // For top/bottom, we cap height to a reasonable 150px to prevent squeezing the plot
                    let size = spec.estimate_size(theme, 150.0);

                    if current_row_w + size.width > max_w && current_row_w > 0.0 {
                        total_height += current_row_h + block_gap;
                        current_row_h = size.height;
                        current_row_w = size.width;
                    } else {
                        current_row_h = f64::max(current_row_h, size.height);
                        current_row_w += size.width + block_gap;
                    }
                }
                total_height += current_row_h;

                // Defense for vertical space
                let min_panel_h = f64::max(100.0, canvas_h * 0.2);
                let max_allowed_legend_h = (canvas_h - min_panel_h - 60.0).max(0.0);
                
                let final_h = f64::min(total_height, max_allowed_legend_h);
                let reserve = final_h + margin_gap;

                if position == LegendPosition::Top { constraints.top = reserve; }
                else { constraints.bottom = reserve; }
            }
            _ => {}
        }
        constraints
    }

    /// Calculates the space required for axis ticks, labels, and titles.
    pub fn calculate_axis_constraints(
        ctx: &SharedRenderingContext,
        theme: &Theme,
        x_label: &str,
        y_label: &str,
    ) -> AxisLayoutConstraints {
        let mut constraints = AxisLayoutConstraints::default();
        let coord = ctx.coord;
        let is_flipped = coord.is_flipped();

        // Calculate Bottom Axis (usually X, but Y if flipped)
        let (b_scale, b_angle, b_title, b_pad) = if is_flipped {
            (coord.get_y_scale(), theme.y_tick_label_angle, y_label, theme.label_padding)
        } else {
            (coord.get_x_scale(), theme.x_tick_label_angle, x_label, theme.label_padding)
        };
        constraints.bottom = Self::estimate_axis_dimension(b_scale, b_angle, b_title, b_pad, theme, true);

        // Calculate Left Axis (usually Y, but X if flipped)
        let (l_scale, l_angle, l_title, l_pad) = if is_flipped {
            (coord.get_x_scale(), theme.x_tick_label_angle, x_label, theme.label_padding)
        } else {
            (coord.get_y_scale(), theme.y_tick_label_angle, y_label, theme.label_padding)
        };
        constraints.left = Self::estimate_axis_dimension(l_scale, l_angle, l_title, l_pad, theme, false);

        constraints
    }

    /// Helper to measure axis required depth based on font size and label rotation.
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

        // Measure the 'depth' impact of labels after rotation
        let max_label_footprint = ticks.iter()
            .map(|t| {
                let w = estimate_text_width(&t.label, theme.tick_label_size);
                let h = theme.tick_label_size;
                if is_horizontal_axis {
                    // For a bottom axis, rotation affects vertical depth
                    w.abs() * angle_rad.sin().abs() + h * angle_rad.cos().abs()
                } else {
                    // For a left axis, rotation affects horizontal depth
                    w.abs() * angle_rad.cos().abs() + h * angle_rad.sin().abs()
                }
            })
            .fold(0.0, f64::max);

        let title_area = if title.is_empty() { 
            0.0 
        } else { 
            theme.label_size + label_padding + 5.0 
        };

        tick_line_len + theme.tick_label_padding + max_label_footprint + title_area + edge_buffer
    }
}