use crate::core::layer::{MarkRenderer, RenderBackend, PolygonConfig};
use crate::core::context::PanelContext;
use crate::chart::Chart;
use crate::mark::bar::MarkBar;
use crate::visual::color::SingleColor;
use crate::error::ChartonError;
use crate::Precision;
use polars::prelude::*;

impl MarkRenderer for Chart<MarkBar> {
    fn render_marks(
        &self,
        backend: &mut dyn RenderBackend,
        context: &PanelContext,
    ) -> Result<(), ChartonError> {
        let df = &self.data.df;
        if df.height() == 0 { return Ok(()); }

        let mark_config = self.mark.as_ref()
            .ok_or_else(|| ChartonError::Mark("MarkBar config missing".into()))?;

        // --- STEP 1: Encoding & Scales ---
        let x_enc = self.encoding.x.as_ref().ok_or(ChartonError::Encoding("X missing".into()))?;
        let y_enc = self.encoding.y.as_ref().ok_or(ChartonError::Encoding("Y missing".into()))?;
        let x_scale = context.coord.get_x_scale();
        let y_scale = context.coord.get_y_scale();

        let is_stacked = y_enc.stack;
        let x_field = &x_enc.field;
        let color_field = self.encoding.color.as_ref().map(|c| c.field.as_str());

        // --- STEP 2: Resolve Coordinate Layout Hints ---
        // Coordinates (Polar vs Cartesian) suggest different default behaviors.
        let hints = context.coord.layout_hints();

        // --- STEP 3: Parameter Resolution ---
        // Use user overrides if present; otherwise, fall back to coordinate hints.
        let eff_width   = mark_config.width.unwrap_or(hints.default_bar_width);
        let eff_spacing = mark_config.spacing.unwrap_or(hints.default_bar_spacing);
        let eff_span    = mark_config.span.unwrap_or(hints.default_bar_span);
        let eff_stroke  = mark_config.stroke.clone().unwrap_or(hints.default_bar_stroke.clone());

        // --- STEP 4: Calculate Unit Step in Normalized Space ---
        let n0 = x_scale.normalize(0.0);
        let n1 = x_scale.normalize(1.0);
        let unit_step_norm = (n1 - n0).abs();

        // --- STEP 5: Grouping & Row-Stub Layout Strategy ---
        // Rule: The layout is driven by the physical row count per X-category.
        // Because of the Cartesian Product in transform_bar_data, every X has the same row count.
        let x_uniques_count = df.column(x_field)?.n_unique()?;
        let total_rows = df.height();
        
        // n_groups: How many marks compete for space in a single X slot.
        // If x == color, n_groups = 1 (Full width sector).
        // If x != color, n_groups > 1 (Side-by-side narrowed bars).
        let n_groups = (total_rows as f64 / x_uniques_count as f64).max(1.0);

        // Logical bar width calculation based on the number of sub-groups.
        let bar_width_data = if is_stacked || n_groups <= 1.0 {
            eff_width.min(eff_span)
        } else {
            eff_span / (n_groups + (n_groups - 1.0) * eff_spacing)
        };

        let bar_width_norm = bar_width_data * unit_step_norm;
        let spacing_norm = bar_width_norm * eff_spacing;

        // --- STEP 6: Partitioning for Rendering ---
        // Self-mapping check: If color field is the same as X, we don't partition
        // because we want to iterate through categories in a single loop to resolve
        // colors per row.
        let is_self_mapping = color_field.map_or(false, |cf| cf == x_field);
        let groups = match color_field {
            Some(col) if !is_self_mapping => df.partition_by_stable([col], true)?,
            _ => vec![df.clone()],
        };

        let mut stack_acc = Vec::new();

        // --- STEP 7: Render Loop ---
        for (group_idx, group_df) in groups.iter().enumerate() {
            // If NOT self-mapping, we resolve color once per group for performance.
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
                // Skip rendering empty placeholders (from Cartesian gap filling) in dodge mode.
                if y_val == 0.0 && !is_stacked { continue; }

                // Resolve Color:
                // If x == color, we resolve for the specific row to get different colors per sector.
                let final_color = if is_self_mapping {
                    let row_df = group_df.slice(i as i64, 1);
                    self.resolve_group_color(&row_df, context, &mark_config.color)?
                } else {
                    group_color_fixed.clone().unwrap_or(mark_config.color.clone())
                };

                let x_tick_n = opt_x_n.unwrap_or(0.0);

                // Calculate Y bounds (Stacked vs Identity)
                let (y_low_n, y_high_n) = if is_stacked {
                    if stack_acc.len() <= i { stack_acc.push(0.0); }
                    let start = stack_acc[i];
                    let end = start + y_val;
                    stack_acc[i] = end;
                    (y_scale.normalize(start), y_scale.normalize(end))
                } else {
                    (y_scale.normalize(0.0), y_scale.normalize(y_val))
                };

                // Offset calculation:
                // Shifts bars side-by-side. If n_groups == 1, offset is 0.0.
                let offset_norm = if !is_stacked && n_groups > 1.0 {
                    (group_idx as f64 - (n_groups - 1.0) / 2.0) * (bar_width_norm + spacing_norm)
                } else {
                    0.0
                };

                let x_center_n = x_tick_n + offset_norm;
                let left_n = x_center_n - bar_width_norm / 2.0;
                let right_n = x_center_n + bar_width_norm / 2.0;

                // --- GEOMETRIC TRANSFORMATION ---
                let rect_path = vec![
                    (left_n, y_low_n),   // Bottom-Left
                    (left_n, y_high_n),  // Top-Left
                    (right_n, y_high_n), // Top-Right
                    (right_n, y_low_n),  // Bottom-Right
                ];

                let pixel_points = if hints.needs_interpolation {
                    // Polar/Geographic: Distort straight normalized lines into curves.
                    context.transform_path(&rect_path, true)
                } else {
                    // Cartesian: Fast vertex-only transformation.
                    rect_path.iter()
                        .map(|(nx, ny)| context.coord.transform(*nx, *ny, &context.panel))
                        .collect::<Vec<_>>()
                };

                backend.draw_polygon(PolygonConfig {
                    points: pixel_points.into_iter()
                        .map(|(x, y)| (x as Precision, y as Precision))
                        .collect(),
                    fill: final_color,
                    stroke: eff_stroke.clone(),
                    stroke_width: mark_config.stroke_width as Precision,
                    fill_opacity: mark_config.opacity as Precision,
                    stroke_opacity: mark_config.opacity as Precision,
                });
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
        fallback: &SingleColor
    ) -> Result<SingleColor, ChartonError> {
        if let Some(ref mapping) = context.spec.aesthetics.color {
            let s = df.column(&mapping.field)?.as_materialized_series();
            let s_trait = mapping.scale_impl.as_ref();
            
            // Normalize the first value of the provided DataFrame slice.
            let norms = s_trait.scale_type().normalize_series(s_trait, &s.head(Some(1)))?;
            let norm = norms.get(0).unwrap_or(0.0);
            
            // Map normalized value to color via palette.
            Ok(s_trait.mapper()
                .map(|m| m.map_to_color(norm, s_trait.logical_max()))
                .unwrap_or_else(|| fallback.clone()))
        } else {
            Ok(fallback.clone())
        }
    }
}
