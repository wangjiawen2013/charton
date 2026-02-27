use super::context::PanelContext;
use super::guide::{GuideSpec, LegendPosition};
use super::utils::estimate_text_width;
use crate::theme::Theme;

/// Physical constraints calculated for axis areas.
#[derive(Default, Debug, Clone, Copy)]
pub struct AxisLayoutConstraints {
    pub bottom: f64,
    pub left: f64,
}

/// Margin reserved on each side of the plot for legend placement.
#[derive(Default, Debug, Clone, Copy)]
pub struct LegendLayoutConstraints {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
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
        canvas_w: f64,
        canvas_h: f64,
        initial_plot_w: f64,
        initial_plot_h: f64,
        margin_gap: f64, // Space between plot panel and the whole legend block
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
                        current_col_w = f64::max(current_col_w, size.width);
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
                let min_panel_w =
                    f64::max(theme.min_panel_size, canvas_w * theme.panel_defense_ratio);
                let max_allowed_legend_w =
                    (canvas_w - min_panel_w - theme.axis_reserve_buffer).max(0.0);

                let final_w = f64::min(total_width, max_allowed_legend_w);
                let reserve = if final_w > 0.0 {
                    final_w + margin_gap
                } else {
                    0.0
                };

                if position == LegendPosition::Right {
                    constraints.right = reserve;
                } else {
                    constraints.left = reserve;
                }
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
                        current_row_h = f64::max(current_row_h, size.height);
                        current_row_w += size.width;

                        if i < specs.len() - 1 {
                            current_row_w += block_gap;
                        }
                    }
                }
                total_height += current_row_h;

                let min_panel_h =
                    f64::max(theme.min_panel_size, canvas_h * theme.panel_defense_ratio);
                let max_allowed_legend_h =
                    (canvas_h - min_panel_h - theme.axis_reserve_buffer).max(0.0);

                let final_h = f64::min(total_height, max_allowed_legend_h);
                let reserve = if final_h > 0.0 {
                    final_h + margin_gap
                } else {
                    0.0
                };

                if position == LegendPosition::Top {
                    constraints.top = reserve;
                } else {
                    constraints.bottom = reserve;
                }
            }
            LegendPosition::None => {
                return constraints;
            }
        }
        constraints
    }

    /// Calculates layout constraints based on predicted axis dimensions.
    ///
    /// This method uses the chart's reference dimensions to estimate the "worst-case"
    /// margin required for labels and titles.
    pub fn calculate_axis_constraints(
        ctx: &PanelContext,
        theme: &Theme,
        reference_width: f64,
        reference_height: f64,
    ) -> AxisLayoutConstraints {
        let mut constraints = AxisLayoutConstraints::default();
        let coord = ctx.coord.clone();
        let is_flipped = coord.is_flipped();

        // 1. Resolve Bottom Axis:
        // Uses X-scale by default; uses Y-scale if the coordinate system is flipped.
        let (b_scale, b_angle, b_title, b_pad) = if is_flipped {
            (
                coord.get_y_scale(),
                theme.y_tick_label_angle,
                coord.get_y_label(),
                theme.label_padding,
            )
        } else {
            (
                coord.get_x_scale(),
                theme.x_tick_label_angle,
                coord.get_x_label(),
                theme.label_padding,
            )
        };
        constraints.bottom = Self::estimate_axis_dimension(
            b_scale,
            b_angle,
            b_title,
            b_pad,
            theme,
            true,
            reference_width,
        );

        // 2. Resolve Left Axis:
        // Uses Y-axis by default, or X-axis if flipped.
        let (l_scale, l_angle, l_title, l_pad) = if is_flipped {
            (
                coord.get_x_scale(),
                theme.x_tick_label_angle,
                coord.get_x_label(),
                theme.label_padding,
            )
        } else {
            (
                coord.get_y_scale(),
                theme.y_tick_label_angle,
                coord.get_y_label(),
                theme.label_padding,
            )
        };
        constraints.left = Self::estimate_axis_dimension(
            l_scale,
            l_angle,
            l_title,
            l_pad,
            theme,
            false,
            reference_height,
        );

        constraints
    }

    /// Estimates the total physical 'depth' required for an axis.
    ///
    /// For a Bottom axis, this represents the total height (from the X-axis line down to the SVG edge).
    /// For a Left axis, this represents the total width (from the Y-axis line left to the SVG edge).
    ///
    /// It accounts for:
    /// 1. Tick mark lines.
    /// 2. Padding between ticks and labels.
    /// 3. The bounding box of rotated labels (using trigonometry).
    /// 4. Padding between labels and the axis title.
    /// 5. The height of the title text itself.
    /// 6. A final safety buffer for the SVG edge.
    fn estimate_axis_dimension(
        scale: &dyn crate::scale::ScaleTrait,
        angle_deg: f64,
        title: &str,
        label_padding: f64, // The padding between labels and title (theme.label_padding)
        theme: &Theme,
        is_horizontal_axis: bool,
        available_space: f64,
    ) -> f64 {
        // Physical constants (should match those used in draw_ticks_and_labels)
        let tick_line_len = 6.0;
        let title_gap = 5.0; // Distance between labels and the title text
        let edge_buffer = 10.0; // Prevents the title from touching the very edge of the SVG
        let angle_rad = angle_deg.to_radians();

        // 1. Predictive Tick Generation
        // We must generate the same number of ticks as the renderer to ensure we
        // measure the actual strings (like "1.0000E7") that will be displayed.
        let final_count = theme.suggest_tick_count(available_space);
        let ticks = scale.ticks(final_count);

        // 2. Compute the physical footprint of the labels
        // Rotated text creates a bounding box. We need the projection of this box
        // onto the axis perpendicular to the chart.
        let max_label_footprint = ticks
            .iter()
            .map(|t| {
                // Approximation of string width based on character weights
                let w = estimate_text_width(&t.label, theme.tick_label_size);
                // The height of the font (cap-height approximation)
                let h = theme.tick_label_size;

                // Formula for the height/width of a rotated rectangle:
                // For Bottom axis (horizontal), we need vertical depth: |w*sin| + |h*cos|
                // For Left axis (vertical), we need horizontal depth: |w*cos| + |h*sin|
                if is_horizontal_axis {
                    w.abs() * angle_rad.sin().abs() + h * angle_rad.cos().abs()
                } else {
                    w.abs() * angle_rad.cos().abs() + h * angle_rad.sin().abs()
                }
            })
            .fold(0.0, f64::max);

        // 3. Title Area Calculation
        // If a title exists, we need to reserve space for the gap and the text height.
        let title_area = if title.is_empty() {
            0.0
        } else {
            // We reserve the full height of the title font (label_size) plus the
            // padding and the gap. This ensures the title doesn't overlap labels
            // and doesn't get clipped by the SVG boundary.
            title_gap + theme.label_size + label_padding
        };

        // 4. Summation of Layout Segments
        // Total Depth = [Tick] + [Padding] + [Label Box] + [Title Area] + [Edge Buffer]
        tick_line_len + theme.tick_label_padding + max_label_footprint + title_area + edge_buffer
    }
}
