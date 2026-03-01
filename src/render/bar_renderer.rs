use crate::Precision;
use crate::chart::Chart;
use crate::core::context::PanelContext;
use crate::core::layer::{MarkRenderer, PolygonConfig, RenderBackend};
use crate::error::ChartonError;
use crate::mark::bar::MarkBar;
use crate::visual::color::SingleColor;
use polars::prelude::*;

impl MarkRenderer for Chart<MarkBar> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df = &self.data.df;
        if df.height() == 0 {
            return Ok(());
        }

        let mark_config = self
            .mark
            .as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkBar config missing".into()))?;

        // --- STEP 1: Encoding & Scales ---
        let x_enc = self
            .encoding
            .x
            .as_ref()
            .ok_or(ChartonError::Encoding("X missing".into()))?;
        let y_enc = self
            .encoding
            .y
            .as_ref()
            .ok_or(ChartonError::Encoding("Y missing".into()))?;
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        let is_stacked = y_enc.stack;
        let x_field = &x_enc.field;
        let color_field = self.encoding.color.as_ref().map(|c| c.field.as_str());

        // PIE MODE: An empty X field indicates a single-axis radial layout.
        let is_pie_mode = x_field.is_empty();

        // --- STEP 2: Coordinate & Mode Detection ---
        let hints = context.coord.layout_hints();
        let is_polar = hints.needs_interpolation;
        let needs_nightingale_sqrt = is_polar && !is_pie_mode;

        // --- STEP 3: Visual Parameter Resolution ---
        let eff_width = mark_config.width.unwrap_or(hints.default_bar_width);
        let eff_spacing = mark_config.spacing.unwrap_or(hints.default_bar_spacing);
        let eff_span = mark_config.span.unwrap_or(hints.default_bar_span);
        let eff_stroke = mark_config
            .stroke
            .unwrap_or(hints.default_bar_stroke);
        let eff_stroke_width = mark_config
            .stroke_width
            .unwrap_or(hints.default_bar_stroke_width);

        // --- STEP 4: X-Axis Step Calculation ---
        let n0 = x_scale.normalize(0.0);
        let n1 = x_scale.normalize(1.0);
        let unit_step_norm = (n1 - n0).abs();

        // --- STEP 5: Grouping & Layout Strategy ---
        let x_uniques_count = df.column(x_field)?.n_unique()?;
        let total_rows = df.height();

        let n_groups = if is_pie_mode {
            1.0
        } else {
            (total_rows as f64 / x_uniques_count as f64).max(1.0)
        };

        let bar_width_data = if is_stacked || n_groups <= 1.0 {
            eff_width.min(eff_span)
        } else {
            eff_span / (n_groups + (n_groups - 1.0) * eff_spacing)
        };

        let bar_width_norm = bar_width_data * unit_step_norm;
        let spacing_norm = bar_width_norm * eff_spacing;

        // --- STEP 6: Partitioning ---
        let is_self_mapping = color_field.is_some_and(|cf| cf == x_field);
        let groups = match color_field {
            Some(col) if !is_self_mapping => df.partition_by_stable([col], true)?,
            _ => vec![df.clone()],
        };

        // --- STEP 7.0: Global Total Calculation (CRITICAL FOR PIE PERCENTAGES) ---
        // We calculate this before the loop to ensure percentages are relative to the whole.
        let global_total = if is_pie_mode {
            df.column(&y_enc.field)?.f64()?.sum().unwrap_or(1.0)
        } else {
            1.0
        };

        let mut stack_acc = Vec::new();

        // --- STEP 7: Render Loop ---
        for (group_idx, group_df) in groups.iter().enumerate() {
            let group_color_fixed = if !is_self_mapping {
                Some(self.resolve_group_color(group_df, context, &mark_config.color)?)
            } else {
                None
            };

            let x_series = group_df.column(x_field)?.as_materialized_series();
            let y_series = group_df.column(&y_enc.field)?.as_materialized_series();

            let x_norms = x_scale.scale_type().normalize_series(x_scale, x_series)?;
            let y_vals: Vec<f64> = y_series.f64()?.into_no_null_iter().collect();

            for (i, (opt_x_n, y_val)) in x_norms.into_iter().zip(y_vals).enumerate() {
                if y_val == 0.0 && !is_stacked {
                    continue;
                }

                let final_color = if is_self_mapping {
                    let row_df = group_df.slice(i as i64, 1);
                    self.resolve_group_color(&row_df, context, &mark_config.color)?
                } else {
                    group_color_fixed
                        .unwrap_or(mark_config.color)
                };

                let x_tick_n = opt_x_n.unwrap_or(0.0);

                // --- RESOLVE Y-BOUNDS (Radius Calculation) ---
                let (y_low_n, y_high_n) = if is_stacked {
                    if stack_acc.len() <= i {
                        stack_acc.push(0.0);
                    }
                    let start = stack_acc[i];
                    let end = start + y_val;
                    stack_acc[i] = end;

                    if needs_nightingale_sqrt {
                        (
                            y_scale.normalize(start).sqrt(),
                            y_scale.normalize(end).sqrt(),
                        )
                    } else {
                        (y_scale.normalize(start), y_scale.normalize(end))
                    }
                } else {
                    let n_val = y_scale.normalize(y_val);
                    let final_n = if needs_nightingale_sqrt {
                        n_val.sqrt()
                    } else {
                        n_val
                    };
                    (y_scale.normalize(0.0), final_n)
                };

                // --- POSITIONING & DODGING ---
                let offset_norm = if !is_stacked && n_groups > 1.0 {
                    (group_idx as f64 - (n_groups - 1.0) / 2.0) * (bar_width_norm + spacing_norm)
                } else {
                    0.0
                };

                let x_center_n = x_tick_n + offset_norm;
                let left_n = x_center_n - bar_width_norm / 2.0;
                let right_n = x_center_n + bar_width_norm / 2.0;

                // --- GEOMETRIC TRANSFORMATION ---
                let rect_path = if is_pie_mode {
                    vec![
                        (y_low_n, left_n),
                        (y_low_n, right_n),
                        (y_high_n, right_n),
                        (y_high_n, left_n),
                    ]
                } else {
                    vec![
                        (left_n, y_low_n),
                        (left_n, y_high_n),
                        (right_n, y_high_n),
                        (right_n, y_low_n),
                    ]
                };

                let pixel_points = if hints.needs_interpolation {
                    context.transform_path(&rect_path, true)
                } else {
                    rect_path
                        .iter()
                        .map(|(nx, ny)| context.coord.transform(*nx, *ny, &context.panel))
                        .collect::<Vec<_>>()
                };

                backend.draw_polygon(PolygonConfig {
                    points: pixel_points
                        .into_iter()
                        .map(|(x, y)| (x as Precision, y as Precision))
                        .collect(),
                    fill: final_color,
                    stroke: eff_stroke,
                    stroke_width: eff_stroke_width as Precision,
                    fill_opacity: mark_config.opacity as Precision,
                    stroke_opacity: mark_config.opacity as Precision,
                });

                // --- 8. PIE/DONUT CENTERED LABELING ---
                if is_pie_mode {
                    let theme = context.spec.theme;

                    // Calculate percentage using the pre-calculated global_total.
                    let percentage = if y_enc.normalize {
                        y_val * 100.0
                    } else {
                        (y_val / global_total) * 100.0
                    };

                    // Only render labels for sectors > 3% to maintain clarity.
                    if percentage > 3.0 {
                        let label_text = format!("{:.1}%", percentage);

                        // Centroid logic: Midpoint of angular and radial spans.
                        let mid_angle_n = (y_low_n + y_high_n) / 2.0;
                        let mid_radius_n = (left_n + right_n) / 2.0;

                        let (lx, ly) =
                            context
                                .coord
                                .transform(mid_angle_n, mid_radius_n, &context.panel);

                        backend.draw_text(crate::core::layer::TextConfig {
                            x: lx as Precision,
                            y: ly as Precision,
                            text: label_text,
                            font_size: (theme.tick_label_size - 1.0) as Precision,
                            font_family: theme.tick_label_family.clone(),
                            color: SingleColor::new("white"),
                            text_anchor: "middle".into(),
                            font_weight: "bold".into(),
                            opacity: mark_config.opacity as Precision,
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

impl Chart<MarkBar> {
    /// Resolves the color for a specific data subset.
    /// In self-mapping mode (x == color), this is called per row.
    /// In grouping mode (x != color), this is called once per partition.
    fn resolve_group_color(
        &self,
        df: &DataFrame,
        context: &PanelContext,
        fallback: &SingleColor,
    ) -> Result<SingleColor, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();

            // Normalize the first value of the provided DataFrame slice.
            let norms = s_trait
                .scale_type()
                .normalize_series(s_trait, &s.head(Some(1)))?;
            let norm = norms.get(0).unwrap_or(0.0);

            // Map normalized value to color via palette.
            Ok(s_trait
                .mapper()
                .map(|m| m.map_to_color(norm, s_trait.logical_max()))
                .unwrap_or_else(|| *fallback))
        } else {
            Ok(*fallback)
        }
    }
}
