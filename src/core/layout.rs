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
    /// Estimates the complete legend footprint using the renderer's wrapping rules.
    pub(crate) fn estimate_legend_extent(
        specs: &[GuideSpec],
        position: LegendPosition,
        available_width: f64,
        available_height: f64,
        theme: &Theme,
    ) -> crate::core::guide::GuideSize {
        if specs.is_empty() || matches!(position, LegendPosition::None) {
            return crate::core::guide::GuideSize::default();
        }

        let block_gap = theme.legend_block_gap;
        let is_horizontal = matches!(position, LegendPosition::Top | LegendPosition::Bottom);
        if matches!(position, LegendPosition::Left | LegendPosition::Right) {
            let mut total_width = 0.0;
            let mut current_width = 0.0;
            let mut current_height = 0.0;
            let mut total_height: f64 = 0.0;

            for (index, spec) in specs.iter().enumerate() {
                let size = spec.estimate_size(theme, available_width, available_height, false);
                if current_height + size.height > available_height && current_height > 0.0 {
                    total_width += current_width + block_gap;
                    total_height = total_height.max(current_height);
                    current_width = size.width;
                    current_height = size.height;
                } else {
                    current_width = current_width.max(size.width);
                    current_height += size.height;
                    if index < specs.len() - 1 {
                        current_height += block_gap;
                    }
                }
            }

            total_width += current_width;
            total_height = total_height.max(current_height);
            return crate::core::guide::GuideSize {
                width: total_width,
                height: total_height,
            };
        }

        let mut total_height = 0.0;
        let mut current_height: f64 = 0.0;
        let mut current_width = 0.0;
        let mut max_width: f64 = 0.0;

        for (index, spec) in specs.iter().enumerate() {
            let size = spec.estimate_size(theme, available_width, available_height, is_horizontal);
            if current_width + size.width > available_width && current_width > 0.0 {
                total_height += current_height + block_gap;
                max_width = max_width.max(current_width);
                current_height = size.height;
                current_width = size.width;
            } else {
                current_height = current_height.max(size.height);
                current_width += size.width;
                if index < specs.len() - 1 {
                    current_width += block_gap;
                }
            }
        }

        total_height += current_height;
        max_width = max_width.max(current_width);
        crate::core::guide::GuideSize {
            width: max_width,
            height: total_height,
        }
    }

    /// Calculates legend margins using a greedy stacking algorithm.
    ///
    /// The logic follows a "Flex-box" style approach:
    /// 1. **Vertical Stacking (Right/Left)**: Legends are stacked in a column.
    ///    If a legend exceeds `initial_plot_h`, a new column is started to the side.
    /// 2. **Horizontal Stacking (Top/Bottom)**: Legends are laid out in a row.
    ///    If a legend exceeds `initial_plot_w`, a new row is started below/above.
    #[allow(clippy::too_many_arguments)]
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

        match position {
            LegendPosition::Right | LegendPosition::Left => {
                let extent = Self::estimate_legend_extent(
                    specs,
                    position,
                    initial_plot_w,
                    initial_plot_h,
                    theme,
                );

                // Safety cap: Prevent legends from consuming too much horizontal space.
                // We ensure the plot panel has a "Defense Floor".
                let min_panel_w =
                    f64::max(theme.min_panel_size, canvas_w * theme.panel_defense_ratio);
                let max_allowed_legend_w =
                    (canvas_w - min_panel_w - theme.axis_reserve_buffer).max(0.0);

                let final_w = f64::min(extent.width, max_allowed_legend_w);
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
                let extent = Self::estimate_legend_extent(
                    specs,
                    position,
                    initial_plot_w,
                    initial_plot_h,
                    theme,
                );

                let min_panel_h =
                    f64::max(theme.min_panel_size, canvas_h * theme.panel_defense_ratio);
                let max_allowed_legend_h =
                    (canvas_h - min_panel_h - theme.axis_reserve_buffer).max(0.0);

                let final_h = f64::min(extent.height, max_allowed_legend_h);
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
        let ticks = scale.suggest_ticks(final_count);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::aesthetics::AestheticMapping;
    use crate::scale::mapper::VisualMapper;
    use crate::scale::{Expansion, Scale, ScaleDomain, create_scale};

    fn legend_specs() -> Vec<GuideSpec> {
        let scale = create_scale(
            &Scale::Discrete,
            ScaleDomain::Discrete(vec!["A".into(), "B".into()]),
            Expansion {
                mult: (0.0, 0.0),
                add: (0.0, 0.0),
            },
            Some(VisualMapper::new_shape_default()),
        )
        .unwrap();

        vec![GuideSpec::new(
            "group".into(),
            ScaleDomain::Discrete(vec!["A".into(), "B".into()]),
            vec![AestheticMapping {
                field: "group".into(),
                scale_impl: scale,
            }],
        )]
    }

    #[test]
    fn legend_constraints_use_the_requested_side() {
        let specs = legend_specs();
        let theme = Theme::default();
        let args = (500.0, 400.0, 400.0, 300.0, 8.0, &theme);

        let left = LayoutEngine::calculate_legend_constraints(
            &specs,
            LegendPosition::Left,
            args.0,
            args.1,
            args.2,
            args.3,
            args.4,
            args.5,
        );
        let right = LayoutEngine::calculate_legend_constraints(
            &specs,
            LegendPosition::Right,
            args.0,
            args.1,
            args.2,
            args.3,
            args.4,
            args.5,
        );
        let top = LayoutEngine::calculate_legend_constraints(
            &specs,
            LegendPosition::Top,
            args.0,
            args.1,
            args.2,
            args.3,
            args.4,
            args.5,
        );
        let bottom = LayoutEngine::calculate_legend_constraints(
            &specs,
            LegendPosition::Bottom,
            args.0,
            args.1,
            args.2,
            args.3,
            args.4,
            args.5,
        );

        assert!(left.left > 0.0 && left.right == 0.0 && left.top == 0.0);
        assert!(right.right > 0.0 && right.left == 0.0 && right.top == 0.0);
        assert!(top.top > 0.0 && top.bottom == 0.0 && top.left == 0.0);
        assert!(bottom.bottom > 0.0 && bottom.top == 0.0 && bottom.left == 0.0);
    }
}
